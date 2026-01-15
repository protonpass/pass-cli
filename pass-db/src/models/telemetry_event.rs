use crate::DbConnection;
use anyhow::{Context, Result};
use pass_domain::{TelemetryEvent, TelemetryEventData};
use rusqlite::{OptionalExtension, Row, params};

#[derive(Debug, Clone)]
pub struct TelemetryEventModel {
    pub id: i64,
    pub timestamp: i64,
    pub event_type: String,
    pub extra_data: Option<String>,
    pub user_id: Option<String>,
}

impl From<TelemetryEventModel> for TelemetryEventData {
    fn from(model: TelemetryEventModel) -> Self {
        let dimensions = model
            .extra_data
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default();

        TelemetryEventData {
            event_type: model.event_type,
            dimensions,
            user_id: model.user_id,
            timestamp: model.timestamp,
        }
    }
}

impl TelemetryEventModel {
    pub fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(TelemetryEventModel {
            id: row.get("id")?,
            timestamp: row.get("timestamp")?,
            event_type: row.get("event_type")?,
            extra_data: row.get("extra_data")?,
            user_id: row.get("user_id")?,
        })
    }

    pub async fn insert(
        conn: &DbConnection,
        event: &dyn TelemetryEvent,
        user_id: Option<String>,
    ) -> Result<i64> {
        let event_type = event.event_type();
        let dimensions = event.dimensions();
        let extra_data =
            serde_json::to_string(&dimensions).context("Failed to serialize dimensions")?;
        let timestamp = jiff::Timestamp::now().as_second();

        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO telemetry_events (timestamp, event_type, extra_data, user_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![timestamp, event_type, extra_data, user_id],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await?
    }

    pub async fn get_all(conn: &DbConnection) -> Result<Vec<TelemetryEventData>> {
        conn.interact(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, event_type, extra_data, user_id
                 FROM telemetry_events
                 ORDER BY timestamp ASC",
            )?;

            let events = stmt
                .query_map([], |row| Ok(TelemetryEventModel::from_row(row)))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect::<Result<Vec<TelemetryEventModel>>>()?;

            Ok(events.into_iter().map(TelemetryEventData::from).collect())
        })
        .await?
    }

    pub async fn get_by_user_id(
        conn: &DbConnection,
        user_id: &str,
    ) -> Result<Vec<TelemetryEventData>> {
        let user_id = user_id.to_string();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, event_type, extra_data, user_id
                 FROM telemetry_events
                 WHERE user_id = ?1
                 ORDER BY timestamp ASC",
            )?;

            let events = stmt
                .query_map([&user_id], |row| Ok(TelemetryEventModel::from_row(row)))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect::<Result<Vec<TelemetryEventModel>>>()?;

            Ok(events.into_iter().map(TelemetryEventData::from).collect())
        })
        .await?
    }

    pub async fn get_by_id(conn: &DbConnection, id: i64) -> Result<Option<TelemetryEventModel>> {
        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, event_type, extra_data, user_id
                 FROM telemetry_events
                 WHERE id = ?1",
            )?;

            let event = stmt
                .query_row([id], |row| Ok(TelemetryEventModel::from_row(row)))
                .optional()?
                .transpose()?;

            Ok(event)
        })
        .await?
    }

    pub async fn delete_by_user_id(conn: &DbConnection, user_id: &str) -> Result<usize> {
        let user_id = user_id.to_string();
        conn.interact(move |conn| {
            let count = conn.execute(
                "DELETE FROM telemetry_events WHERE user_id = ?1",
                [&user_id],
            )?;
            Ok(count)
        })
        .await?
    }

    pub async fn delete_all(conn: &DbConnection) -> Result<usize> {
        conn.interact(|conn| {
            let count = conn.execute("DELETE FROM telemetry_events", [])?;
            Ok(count)
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_db;
    use pass_domain::ItemType;
    use std::collections::HashMap;

    struct TestTelemetryEvent1 {
        item_type: ItemType,
    }

    impl TelemetryEvent for TestTelemetryEvent1 {
        fn event_type(&self) -> String {
            "TestTelemetryEvent1".to_string()
        }

        fn dimensions(&self) -> HashMap<String, String> {
            let mut res = HashMap::new();
            res.insert("type".to_string(), self.item_type.as_str().to_string());
            res
        }
    }

    struct TestTelemetryEvent2;
    impl TelemetryEvent for TestTelemetryEvent2 {
        fn event_type(&self) -> String {
            "TestTelemetryEvent2".to_string()
        }
    }

    struct TestTelemetryEvent3;
    impl TelemetryEvent for TestTelemetryEvent3 {
        fn event_type(&self) -> String {
            "TestTelemetryEvent3".to_string()
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TestTelemetryEvent1 {
            item_type: ItemType::Login,
        };
        let user_id = Some("user123".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        assert!(id > 0);

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.event_type, event.event_type());

        // Verify the dimensions are stored as JSON
        let dimensions: HashMap<String, String> =
            serde_json::from_str(retrieved.extra_data.as_ref().unwrap()).unwrap();
        assert_eq!(event.dimensions(), dimensions);
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_event_with_no_dimensions() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TestTelemetryEvent2;
        let user_id = Some("user111".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, event.event_type());
        assert_eq!(retrieved.extra_data, Some("{}".to_string())); // Converted to empty object
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_all_item_types() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let item_types = vec![
            ItemType::Note,
            ItemType::Login,
            ItemType::Alias,
            ItemType::CreditCard,
            ItemType::Identity,
            ItemType::SshKey,
            ItemType::Wifi,
            ItemType::Custom,
        ];

        for item_type in item_types {
            let event = TestTelemetryEvent1 {
                item_type: item_type.clone(),
            };
            let id = TelemetryEventModel::insert(&conn, &event, None)
                .await
                .unwrap();
            assert!(id > 0);

            let retrieved = TelemetryEventModel::get_by_id(&conn, id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(retrieved.event_type, event.event_type());
            assert!(retrieved.extra_data.is_some());

            let extra_data = retrieved.extra_data.unwrap();
            let parsed: HashMap<String, String> = serde_json::from_str(&extra_data).unwrap();
            assert_eq!(event.dimensions(), parsed);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_all_events() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Login,
            },
            None,
        )
        .await
        .unwrap();
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
            .await
            .unwrap();
        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Note,
            },
            None,
        )
        .await
        .unwrap();

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user1 = "user1";
        let user2 = "user2";

        // Insert events for user1
        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Login,
            },
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, Some(user1.to_string()))
            .await
            .unwrap();

        // Insert events for user2
        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Note,
            },
            Some(user2.to_string()),
        )
        .await
        .unwrap();

        // Insert event with no user
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
            .await
            .unwrap();

        let user1_events = TelemetryEventModel::get_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(user1_events.len(), 2);
        assert!(
            user1_events
                .iter()
                .all(|e| e.user_id.as_deref() == Some(user1))
        );

        let user2_events = TelemetryEventModel::get_by_user_id(&conn, user2)
            .await
            .unwrap();
        assert_eq!(user2_events.len(), 1);
        assert!(
            user2_events
                .iter()
                .all(|e| e.user_id.as_deref() == Some(user2))
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_id_nonexistent() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let result = TelemetryEventModel::get_by_id(&conn, 999999).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_all() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert some events
        for _ in 0..5 {
            TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
                .await
                .unwrap();
        }

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 5);

        let deleted_count = TelemetryEventModel::delete_all(&conn).await.unwrap();
        assert_eq!(deleted_count, 5);

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_model_to_event_data() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let event = TestTelemetryEvent1 {
            item_type: ItemType::Alias,
        };

        let id = TelemetryEventModel::insert(&conn, &event, Some("user123".to_string()))
            .await
            .unwrap();

        let model = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        let event_data: TelemetryEventData = model.into();

        assert_eq!(event_data.event_type, event.event_type());
        assert_eq!(
            event_data.dimensions.get("type"),
            Some(&"alias".to_string())
        );
        assert_eq!(event_data.user_id, Some("user123".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_events_ordered_by_timestamp() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert events with slight delays to ensure different timestamps
        let event_1 = TestTelemetryEvent1 {
            item_type: ItemType::Note,
        };
        let _id1 = TelemetryEventModel::insert(&conn, &event_1, None)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event_2 = TestTelemetryEvent2;
        let _id2 = TelemetryEventModel::insert(&conn, &event_2, None)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let event_3 = TestTelemetryEvent3;
        let _id3 = TelemetryEventModel::insert(&conn, &event_3, None)
            .await
            .unwrap();

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();

        assert_eq!(all_events.len(), 3);
        // Verify event types match insertion order
        assert_eq!(all_events[0].event_type, event_1.event_type());
        assert_eq!(all_events[1].event_type, event_2.event_type());
        assert_eq!(all_events[2].event_type, event_3.event_type());

        // Verify timestamps are in ascending order
        assert!(all_events[0].timestamp <= all_events[1].timestamp);
        assert!(all_events[1].timestamp <= all_events[2].timestamp);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let event = TestTelemetryEvent1 {
            item_type: ItemType::Wifi,
        };
        let user_id = Some("user999".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        assert!(id > 0);

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, event.event_type());
        let dimensions: std::collections::HashMap<String, String> =
            serde_json::from_str(retrieved.extra_data.as_ref().unwrap()).unwrap();
        assert_eq!(dimensions.get("type"), Some(&"wifi".to_string()));
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_all() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert events using the connection
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
            .await
            .unwrap();

        TelemetryEventModel::insert(&conn, &TestTelemetryEvent3, None)
            .await
            .unwrap();

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();

        assert_eq!(all_events.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_user_id_connection() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = "test_user";

        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Custom,
            },
            Some(user_id.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
            .await
            .unwrap();

        let user_events = TelemetryEventModel::get_by_user_id(&conn, user_id)
            .await
            .unwrap();

        assert_eq!(user_events.len(), 1);
        assert_eq!(user_events[0].user_id.as_deref(), Some(user_id));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_user_id_with_special_characters() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let special_user_ids = vec![
            "user@example.com",
            "user-with-dashes",
            "user_with_underscores",
            "user123",
            "UsErWiThMiXeDcAsE",
        ];

        for user_id in special_user_ids {
            let id =
                TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, Some(user_id.to_string()))
                    .await
                    .unwrap();

            let retrieved = TelemetryEventModel::get_by_id(&conn, id)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(retrieved.user_id.as_deref(), Some(user_id));

            let user_events = TelemetryEventModel::get_by_user_id(&conn, user_id)
                .await
                .unwrap();

            assert_eq!(1, user_events.len());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user1 = "user1";
        let user2 = "user2";

        // Insert events for user1
        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Login,
            },
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, Some(user1.to_string()))
            .await
            .unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::Note,
            },
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        // Insert events for user2
        TelemetryEventModel::insert(
            &conn,
            &TestTelemetryEvent1 {
                item_type: ItemType::CreditCard,
            },
            Some(user2.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, Some(user2.to_string()))
            .await
            .unwrap();

        // Insert event with no user
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent3, None)
            .await
            .unwrap();

        // Verify initial state
        let user1_events = TelemetryEventModel::get_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(user1_events.len(), 3);

        let user2_events = TelemetryEventModel::get_by_user_id(&conn, user2)
            .await
            .unwrap();
        assert_eq!(user2_events.len(), 2);

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 6);

        // Delete user1 events
        let deleted_count = TelemetryEventModel::delete_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(deleted_count, 3);

        // Verify user1 events are gone
        let user1_events = TelemetryEventModel::get_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(user1_events.len(), 0);

        // Verify user2 events and null user events are still there
        let user2_events = TelemetryEventModel::get_by_user_id(&conn, user2)
            .await
            .unwrap();
        assert_eq!(user2_events.len(), 2);

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id_nonexistent() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert some events
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, Some("user1".to_string()))
            .await
            .unwrap();

        // Delete non-existent user
        let deleted_count = TelemetryEventModel::delete_by_user_id(&conn, "nonexistent_user")
            .await
            .unwrap();
        assert_eq!(deleted_count, 0);

        // Verify original events are still there
        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id_does_not_delete_null_users() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = "user_to_delete";

        // Insert events with user_id
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, Some(user_id.to_string()))
            .await
            .unwrap();

        // Insert events with null user_id
        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
            .await
            .unwrap();

        TelemetryEventModel::insert(&conn, &TestTelemetryEvent2, None)
            .await
            .unwrap();

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 3);

        // Delete by user_id
        let deleted_count = TelemetryEventModel::delete_by_user_id(&conn, user_id)
            .await
            .unwrap();
        assert_eq!(deleted_count, 1);

        // Verify null user events are still there
        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 2);
        assert!(all_events.iter().all(|e| e.user_id.is_none()));
    }
}
