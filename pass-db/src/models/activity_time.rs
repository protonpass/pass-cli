/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::DbConnection;
use anyhow::Result;
use rusqlite::{OptionalExtension, Row, params};

#[derive(Debug, Clone, PartialEq)]
pub struct ActivityTimeModel {
    pub user_id: Option<String>,
    pub activity: String,
    pub timestamp: i64,
}

impl ActivityTimeModel {
    pub fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(ActivityTimeModel {
            user_id: row.get("user_id")?,
            activity: row.get("activity")?,
            timestamp: row.get("timestamp")?,
        })
    }

    pub async fn upsert(
        conn: &DbConnection,
        user_id: Option<String>,
        activity: &str,
        timestamp: i64,
    ) -> Result<()> {
        let activity = activity.to_string();
        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO activity_time (user_id, activity, timestamp) VALUES (?1, ?2, ?3)",
                params![user_id, activity, timestamp],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn get(
        conn: &DbConnection,
        user_id: Option<&str>,
        activity: &str,
    ) -> Result<Option<ActivityTimeModel>> {
        let user_id = user_id.map(|s| s.to_string());
        let activity = activity.to_string();
        conn.interact(move |conn| {
            let result = if let Some(ref uid) = user_id {
                let mut stmt = conn.prepare(
                    "SELECT user_id, activity, timestamp FROM activity_time WHERE user_id = ?1 AND activity = ?2",
                )?;
                stmt.query_row(params![uid, &activity], |row| {
                    Ok(ActivityTimeModel::from_row(row))
                })
                .optional()?
                .transpose()?
            } else {
                let mut stmt = conn.prepare(
                    "SELECT user_id, activity, timestamp FROM activity_time WHERE user_id IS NULL AND activity = ?1",
                )?;
                stmt.query_row(params![&activity], |row| {
                    Ok(ActivityTimeModel::from_row(row))
                })
                .optional()?
                .transpose()?
            };

            Ok(result)
        })
        .await?
    }

    pub async fn get_by_user_id(
        conn: &DbConnection,
        user_id: Option<&str>,
    ) -> Result<Vec<ActivityTimeModel>> {
        let user_id = user_id.map(|s| s.to_string());
        conn.interact(move |conn| {
            let records = if let Some(ref uid) = user_id {
                let mut stmt = conn.prepare(
                    "SELECT user_id, activity, timestamp FROM activity_time WHERE user_id = ?1 ORDER BY timestamp DESC",
                )?;
                stmt.query_map([uid], |row| Ok(ActivityTimeModel::from_row(row)))?
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?
            } else {
                let mut stmt = conn.prepare(
                    "SELECT user_id, activity, timestamp FROM activity_time WHERE user_id IS NULL ORDER BY timestamp DESC",
                )?;
                stmt.query_map([], |row| Ok(ActivityTimeModel::from_row(row)))?
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .collect::<Result<Vec<_>>>()?
            };

            Ok(records)
        })
        .await?
    }

    pub async fn get_all(conn: &DbConnection) -> Result<Vec<ActivityTimeModel>> {
        conn.interact(|conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, activity, timestamp FROM activity_time ORDER BY timestamp DESC",
            )?;

            let records = stmt
                .query_map([], |row| Ok(ActivityTimeModel::from_row(row)))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect::<Result<Vec<_>>>()?;

            Ok(records)
        })
        .await?
    }

    /// Deletes an activity time record for a specific user and activity
    pub async fn delete(
        conn: &DbConnection,
        user_id: Option<&str>,
        activity: &str,
    ) -> Result<usize> {
        let user_id = user_id.map(|s| s.to_string());
        let activity = activity.to_string();
        conn.interact(move |conn| {
            let count = if let Some(ref uid) = user_id {
                conn.execute(
                    "DELETE FROM activity_time WHERE user_id = ?1 AND activity = ?2",
                    params![uid, &activity],
                )?
            } else {
                conn.execute(
                    "DELETE FROM activity_time WHERE user_id IS NULL AND activity = ?1",
                    params![&activity],
                )?
            };
            Ok(count)
        })
        .await?
    }

    pub async fn delete_by_user_id(conn: &DbConnection, user_id: Option<&str>) -> Result<usize> {
        let user_id = user_id.map(|s| s.to_string());
        conn.interact(move |conn| {
            let count = if let Some(ref uid) = user_id {
                conn.execute("DELETE FROM activity_time WHERE user_id = ?1", [uid])?
            } else {
                conn.execute("DELETE FROM activity_time WHERE user_id IS NULL", [])?
            };
            Ok(count)
        })
        .await?
    }

    pub async fn delete_all(conn: &DbConnection) -> Result<usize> {
        conn.interact(|conn| {
            let count = conn.execute("DELETE FROM activity_time", [])?;
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
    async fn test_upsert_and_get() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = Some("user123".to_string());
        let activity = "login";
        let timestamp = 1234567890;

        ActivityTimeModel::upsert(&conn, user_id.clone(), activity, timestamp)
            .await
            .unwrap();

        let retrieved = ActivityTimeModel::get(&conn, user_id.as_deref(), activity)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.activity, activity);
        assert_eq!(retrieved.timestamp, timestamp);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_updates_existing() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = Some("user456".to_string());
        let activity = "telemetry_sent";
        let timestamp1 = 1000000000;
        let timestamp2 = 2000000000;

        // Insert first time
        ActivityTimeModel::upsert(&conn, user_id.clone(), activity, timestamp1)
            .await
            .unwrap();

        let retrieved = ActivityTimeModel::get(&conn, user_id.as_deref(), activity)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.timestamp, timestamp1);

        // Update with new timestamp
        ActivityTimeModel::upsert(&conn, user_id.clone(), activity, timestamp2)
            .await
            .unwrap();

        let retrieved = ActivityTimeModel::get(&conn, user_id.as_deref(), activity)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.timestamp, timestamp2);

        // Verify only one record exists
        let all_records = ActivityTimeModel::get_by_user_id(&conn, user_id.as_deref())
            .await
            .unwrap();
        assert_eq!(all_records.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_nonexistent() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let result = ActivityTimeModel::get(&conn, Some("nonexistent"), "activity")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user1 = Some("user1".to_string());
        let user2 = Some("user2".to_string());

        // Insert multiple activities for user1
        ActivityTimeModel::upsert(&conn, user1.clone(), "login", 1000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, user1.clone(), "logout", 2000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, user1.clone(), "telemetry_sent", 3000)
            .await
            .unwrap();

        // Insert activity for user2
        ActivityTimeModel::upsert(&conn, user2.clone(), "login", 4000)
            .await
            .unwrap();

        // Insert activity with null user
        ActivityTimeModel::upsert(&conn, None, "system_event", 5000)
            .await
            .unwrap();

        let user1_records = ActivityTimeModel::get_by_user_id(&conn, user1.as_deref())
            .await
            .unwrap();
        assert_eq!(user1_records.len(), 3);
        assert!(
            user1_records
                .iter()
                .all(|r| r.user_id.as_deref() == Some("user1"))
        );

        let user2_records = ActivityTimeModel::get_by_user_id(&conn, user2.as_deref())
            .await
            .unwrap();
        assert_eq!(user2_records.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_all() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        ActivityTimeModel::upsert(&conn, Some("user1".to_string()), "activity1", 1000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, Some("user2".to_string()), "activity2", 2000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, None, "activity3", 3000)
            .await
            .unwrap();

        let all_records = ActivityTimeModel::get_all(&conn).await.unwrap();
        assert_eq!(all_records.len(), 3);

        // Verify records are ordered by timestamp DESC
        assert!(all_records[0].timestamp >= all_records[1].timestamp);
        assert!(all_records[1].timestamp >= all_records[2].timestamp);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = Some("user789".to_string());
        let activity = "test_activity";

        ActivityTimeModel::upsert(&conn, user_id.clone(), activity, 1000)
            .await
            .unwrap();

        let retrieved = ActivityTimeModel::get(&conn, user_id.as_deref(), activity)
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let deleted_count = ActivityTimeModel::delete(&conn, user_id.as_deref(), activity)
            .await
            .unwrap();
        assert_eq!(deleted_count, 1);

        let retrieved = ActivityTimeModel::get(&conn, user_id.as_deref(), activity)
            .await
            .unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_nonexistent() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let deleted_count = ActivityTimeModel::delete(&conn, Some("nonexistent"), "activity")
            .await
            .unwrap();
        assert_eq!(deleted_count, 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user1 = Some("user1".to_string());
        let user2 = Some("user2".to_string());

        // Insert multiple activities for user1
        ActivityTimeModel::upsert(&conn, user1.clone(), "activity1", 1000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, user1.clone(), "activity2", 2000)
            .await
            .unwrap();

        // Insert activity for user2
        ActivityTimeModel::upsert(&conn, user2.clone(), "activity3", 3000)
            .await
            .unwrap();

        // Insert activity with null user
        ActivityTimeModel::upsert(&conn, None, "activity4", 4000)
            .await
            .unwrap();

        let deleted_count = ActivityTimeModel::delete_by_user_id(&conn, user1.as_deref())
            .await
            .unwrap();
        assert_eq!(deleted_count, 2);

        let user1_records = ActivityTimeModel::get_by_user_id(&conn, user1.as_deref())
            .await
            .unwrap();
        assert_eq!(user1_records.len(), 0);

        let all_records = ActivityTimeModel::get_all(&conn).await.unwrap();
        assert_eq!(all_records.len(), 2); // user2 and null user remain
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_all() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        ActivityTimeModel::upsert(&conn, Some("user1".to_string()), "activity1", 1000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, Some("user2".to_string()), "activity2", 2000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, None, "activity3", 3000)
            .await
            .unwrap();

        let all_records = ActivityTimeModel::get_all(&conn).await.unwrap();
        assert_eq!(all_records.len(), 3);

        let deleted_count = ActivityTimeModel::delete_all(&conn).await.unwrap();
        assert_eq!(deleted_count, 3);

        let all_records = ActivityTimeModel::get_all(&conn).await.unwrap();
        assert_eq!(all_records.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_null_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let activity = "system_activity";
        let timestamp = 9999999999;

        ActivityTimeModel::upsert(&conn, None, activity, timestamp)
            .await
            .unwrap();

        let retrieved = ActivityTimeModel::get(&conn, None, activity)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.user_id, None);
        assert_eq!(retrieved.activity, activity);
        assert_eq!(retrieved.timestamp, timestamp);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_multiple_users_same_activity() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let activity = "login";
        let user1 = Some("user1".to_string());
        let user2 = Some("user2".to_string());

        ActivityTimeModel::upsert(&conn, user1.clone(), activity, 1000)
            .await
            .unwrap();
        ActivityTimeModel::upsert(&conn, user2.clone(), activity, 2000)
            .await
            .unwrap();

        let user1_record = ActivityTimeModel::get(&conn, user1.as_deref(), activity)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user1_record.timestamp, 1000);

        let user2_record = ActivityTimeModel::get(&conn, user2.as_deref(), activity)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user2_record.timestamp, 2000);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_activity_with_special_characters() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let special_activities = vec![
            "activity-with-dashes",
            "activity_with_underscores",
            "activity.with.dots",
            "activity:with:colons",
            "UPPERCASE_ACTIVITY",
        ];

        for activity in &special_activities {
            ActivityTimeModel::upsert(&conn, Some("user".to_string()), activity, 1000)
                .await
                .unwrap();

            let retrieved = ActivityTimeModel::get(&conn, Some("user"), activity)
                .await
                .unwrap()
                .unwrap();

            assert_eq!(retrieved.activity, *activity);
        }
    }
}
