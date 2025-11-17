use crate::DbConnection;
use anyhow::{Result, anyhow};
use pass_domain::{ItemType, TelemetryEvent};
use rusqlite::{OptionalExtension, Row, params};

#[derive(Debug, Clone)]
pub struct TelemetryEventModel {
    pub id: i64,
    pub timestamp: i64,
    pub event_type: String,
    pub extra_data: Option<String>,
    pub user_id: Option<String>,
}

impl TelemetryEventModel {
    fn parse_item_type(s: &str) -> Option<ItemType> {
        match s {
            "note" => Some(ItemType::Note),
            "login" => Some(ItemType::Login),
            "alias" => Some(ItemType::Alias),
            "credit_card" => Some(ItemType::CreditCard),
            "identity" => Some(ItemType::Identity),
            "ssh_key" => Some(ItemType::SshKey),
            "wifi" => Some(ItemType::Wifi),
            "custom" => Some(ItemType::Custom),
            _ => None,
        }
    }
}

impl TryFrom<TelemetryEventModel> for TelemetryEvent {
    type Error = anyhow::Error;

    fn try_from(row: TelemetryEventModel) -> Result<Self, Self::Error> {
        let event = match row.event_type.as_str() {
            "item_created" => {
                let item_type_str = row
                    .extra_data
                    .as_deref()
                    .ok_or_else(|| anyhow!("item_created event missing extra_data"))?;
                let item_type = TelemetryEventModel::parse_item_type(item_type_str)
                    .ok_or_else(|| anyhow!("unknown item_type: {}", item_type_str))?;
                TelemetryEvent::ItemCreated { item_type }
            }
            "item_updated" => {
                let item_type_str = row
                    .extra_data
                    .as_deref()
                    .ok_or_else(|| anyhow!("item_updated event missing extra_data"))?;
                let item_type = TelemetryEventModel::parse_item_type(item_type_str)
                    .ok_or_else(|| anyhow!("unknown item_type: {}", item_type_str))?;
                TelemetryEvent::ItemUpdated { item_type }
            }
            "item_deleted" => {
                let item_type_str = row
                    .extra_data
                    .as_deref()
                    .ok_or_else(|| anyhow!("item_deleted event missing extra_data"))?;
                let item_type = TelemetryEventModel::parse_item_type(item_type_str)
                    .ok_or_else(|| anyhow!("unknown item_type: {}", item_type_str))?;
                TelemetryEvent::ItemDeleted { item_type }
            }
            "item_moved" => {
                let item_type_str = row
                    .extra_data
                    .as_deref()
                    .ok_or_else(|| anyhow!("item_moved event missing extra_data"))?;
                let item_type = TelemetryEventModel::parse_item_type(item_type_str)
                    .ok_or_else(|| anyhow!("unknown item_type: {}", item_type_str))?;
                TelemetryEvent::ItemMoved { item_type }
            }
            "vault_created" => TelemetryEvent::VaultCreated,
            "vault_updated" => TelemetryEvent::VaultUpdated,
            "vault_deleted" => TelemetryEvent::VaultDeleted,
            "command" => {
                let command = row
                    .extra_data
                    .ok_or_else(|| anyhow!("Command missing extra_data"))?;
                TelemetryEvent::Command { command }
            }
            other => return Err(anyhow!("unknown event_type: {}", other)),
        };

        Ok(event)
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

    fn item_type_to_string(item_type: &ItemType) -> &'static str {
        match item_type {
            ItemType::Note => "note",
            ItemType::Login => "login",
            ItemType::Alias => "alias",
            ItemType::CreditCard => "credit_card",
            ItemType::Identity => "identity",
            ItemType::SshKey => "ssh_key",
            ItemType::Wifi => "wifi",
            ItemType::Custom => "custom",
        }
    }

    pub async fn insert(
        conn: &DbConnection,
        event: &TelemetryEvent,
        user_id: Option<String>,
    ) -> Result<i64> {
        let event_type = event.event_type().to_string();
        let extra_data = match event {
            TelemetryEvent::ItemCreated { item_type }
            | TelemetryEvent::ItemUpdated { item_type }
            | TelemetryEvent::ItemDeleted { item_type }
            | TelemetryEvent::ItemMoved { item_type } => {
                Some(Self::item_type_to_string(item_type).to_string())
            }
            TelemetryEvent::Command { command } => Some(command.to_string()),
            _ => None,
        };
        let timestamp = chrono::Utc::now().timestamp();

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

    pub async fn get_all(conn: &DbConnection) -> Result<Vec<TelemetryEventModel>> {
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
                .collect::<Result<Vec<_>>>()?;

            Ok(events)
        })
        .await?
    }

    pub async fn get_by_user_id(
        conn: &DbConnection,
        user_id: &str,
    ) -> Result<Vec<TelemetryEventModel>> {
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
                .collect::<Result<Vec<_>>>()?;

            Ok(events)
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_item_created_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::ItemCreated {
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
        assert_eq!(retrieved.event_type, "item_created");
        assert_eq!(retrieved.extra_data, Some("login".to_string()));
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_item_updated_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::ItemUpdated {
            item_type: ItemType::Note,
        };
        let user_id = Some("user456".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, "item_updated");
        assert_eq!(retrieved.extra_data, Some("note".to_string()));
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_item_deleted_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::ItemDeleted {
            item_type: ItemType::CreditCard,
        };
        let user_id = None;

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, "item_deleted");
        assert_eq!(retrieved.extra_data, Some("credit_card".to_string()));
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_item_moved_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::ItemMoved {
            item_type: ItemType::SshKey,
        };
        let user_id = Some("user789".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, "item_moved");
        assert_eq!(retrieved.extra_data, Some("ssh_key".to_string()));
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_vault_created_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::VaultCreated;
        let user_id = Some("user111".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, "vault_created");
        assert_eq!(retrieved.extra_data, None);
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_vault_updated_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::VaultUpdated;
        let user_id = None;

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, "vault_updated");
        assert_eq!(retrieved.extra_data, None);
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_and_retrieve_vault_deleted_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();
        let event = TelemetryEvent::VaultDeleted;
        let user_id = Some("user222".to_string());

        let id = TelemetryEventModel::insert(&conn, &event, user_id.clone())
            .await
            .unwrap();

        let retrieved = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.event_type, "vault_deleted");
        assert_eq!(retrieved.extra_data, None);
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
            let event = TelemetryEvent::ItemCreated {
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
            assert_eq!(retrieved.event_type, "item_created");
            assert!(retrieved.extra_data.is_some());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_all_events() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let events = vec![
            TelemetryEvent::ItemCreated {
                item_type: ItemType::Login,
            },
            TelemetryEvent::VaultCreated,
            TelemetryEvent::ItemUpdated {
                item_type: ItemType::Note,
            },
        ];

        for event in &events {
            TelemetryEventModel::insert(&conn, event, None)
                .await
                .unwrap();
        }

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
            &TelemetryEvent::ItemCreated {
                item_type: ItemType::Login,
            },
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::VaultCreated,
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        // Insert events for user2
        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::ItemDeleted {
                item_type: ItemType::Note,
            },
            Some(user2.to_string()),
        )
        .await
        .unwrap();

        // Insert event with no user
        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultUpdated, None)
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
            TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
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
    async fn test_convert_model_to_domain_event() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let event = TelemetryEvent::ItemCreated {
            item_type: ItemType::Alias,
        };

        let id = TelemetryEventModel::insert(&conn, &event, Some("user123".to_string()))
            .await
            .unwrap();

        let model = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        let domain_event: TelemetryEvent = model.try_into().unwrap();

        match domain_event {
            TelemetryEvent::ItemCreated { item_type } => {
                assert_eq!(item_type.as_str(), "alias");
            }
            _ => panic!("Expected ItemCreated event"),
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_convert_all_event_types_to_domain() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let test_cases = vec![
            (
                TelemetryEvent::ItemCreated {
                    item_type: ItemType::Login,
                },
                "item_created",
            ),
            (
                TelemetryEvent::ItemUpdated {
                    item_type: ItemType::Note,
                },
                "item_updated",
            ),
            (
                TelemetryEvent::ItemDeleted {
                    item_type: ItemType::CreditCard,
                },
                "item_deleted",
            ),
            (
                TelemetryEvent::ItemMoved {
                    item_type: ItemType::Identity,
                },
                "item_moved",
            ),
            (TelemetryEvent::VaultCreated, "vault_created"),
            (TelemetryEvent::VaultUpdated, "vault_updated"),
            (TelemetryEvent::VaultDeleted, "vault_deleted"),
        ];

        for (event, expected_type) in test_cases {
            let id = TelemetryEventModel::insert(&conn, &event, None)
                .await
                .unwrap();

            let model = TelemetryEventModel::get_by_id(&conn, id)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(model.event_type, expected_type);

            let domain_event: TelemetryEvent = model.try_into().unwrap();
            assert_eq!(domain_event.event_type(), expected_type);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_timestamp_is_set() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let event = TelemetryEvent::VaultCreated;
        let id = TelemetryEventModel::insert(&conn, &event, None)
            .await
            .unwrap();

        let model = TelemetryEventModel::get_by_id(&conn, id)
            .await
            .unwrap()
            .unwrap();

        // Timestamp should be a reasonable Unix timestamp (not 0, not in the far future)
        assert!(model.timestamp > 0);
        assert!(model.timestamp < chrono::Utc::now().timestamp() + 60);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_events_ordered_by_timestamp() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert events with slight delays to ensure different timestamps
        let id1 = TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let id2 = TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultUpdated, None)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let id3 = TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultDeleted, None)
            .await
            .unwrap();

        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();

        assert_eq!(all_events.len(), 3);
        assert_eq!(all_events[0].id, id1);
        assert_eq!(all_events[1].id, id2);
        assert_eq!(all_events[2].id, id3);

        // Verify timestamps are in ascending order
        assert!(all_events[0].timestamp <= all_events[1].timestamp);
        assert!(all_events[1].timestamp <= all_events[2].timestamp);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let event = TelemetryEvent::ItemCreated {
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

        assert_eq!(retrieved.event_type, "item_created");
        assert_eq!(retrieved.extra_data, Some("wifi".to_string()));
        assert_eq!(retrieved.user_id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_all() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert events using the connection
        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
            .await
            .unwrap();

        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultUpdated, None)
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
            &TelemetryEvent::ItemCreated {
                item_type: ItemType::Custom,
            },
            Some(user_id.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
            .await
            .unwrap();

        let user_events = TelemetryEventModel::get_by_user_id(&conn, user_id)
            .await
            .unwrap();

        assert_eq!(user_events.len(), 1);
        assert_eq!(user_events[0].user_id.as_deref(), Some(user_id));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_inserts_return_unique_ids() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let id1 = TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
            .await
            .unwrap();

        let id2 = TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
            .await
            .unwrap();

        let id3 = TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultCreated, None)
            .await
            .unwrap();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
        assert!(id1 > 0 && id2 > 0 && id3 > 0);
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
            let id = TelemetryEventModel::insert(
                &conn,
                &TelemetryEvent::VaultCreated,
                Some(user_id.to_string()),
            )
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

            assert!(!user_events.is_empty());
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
            &TelemetryEvent::ItemCreated {
                item_type: ItemType::Login,
            },
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::VaultCreated,
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::ItemUpdated {
                item_type: ItemType::Note,
            },
            Some(user1.to_string()),
        )
        .await
        .unwrap();

        // Insert events for user2
        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::ItemDeleted {
                item_type: ItemType::CreditCard,
            },
            Some(user2.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::VaultUpdated,
            Some(user2.to_string()),
        )
        .await
        .unwrap();

        // Insert event with no user
        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultDeleted, None)
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
    async fn test_delete_by_user_id_connection() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = "test_user";

        // Insert events
        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::ItemCreated {
                item_type: ItemType::Alias,
            },
            Some(user_id.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::VaultCreated,
            Some(user_id.to_string()),
        )
        .await
        .unwrap();

        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultUpdated, None)
            .await
            .unwrap();

        // Verify initial state
        let user_events = TelemetryEventModel::get_by_user_id(&conn, user_id)
            .await
            .unwrap();
        assert_eq!(user_events.len(), 2);

        // Delete user events
        let deleted_count = TelemetryEventModel::delete_by_user_id(&conn, user_id)
            .await
            .unwrap();
        assert_eq!(deleted_count, 2);

        // Verify user events are gone
        let user_events = TelemetryEventModel::get_by_user_id(&conn, user_id)
            .await
            .unwrap();
        assert_eq!(user_events.len(), 0);

        // Verify null user event is still there
        let all_events = TelemetryEventModel::get_all(&conn).await.unwrap();
        assert_eq!(all_events.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id_nonexistent() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        // Insert some events
        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::VaultCreated,
            Some("user1".to_string()),
        )
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
        TelemetryEventModel::insert(
            &conn,
            &TelemetryEvent::VaultCreated,
            Some(user_id.to_string()),
        )
        .await
        .unwrap();

        // Insert events with null user_id
        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultUpdated, None)
            .await
            .unwrap();

        TelemetryEventModel::insert(&conn, &TelemetryEvent::VaultDeleted, None)
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
