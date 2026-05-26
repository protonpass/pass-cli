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

use anyhow::Result;
use rusqlite::{Row, params};

#[derive(Debug, Clone)]
pub struct CoreEventCursorModel {
    pub user_id: String,
    pub event_id: String,
    pub updated_at: i64,
}

impl CoreEventCursorModel {
    pub fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(CoreEventCursorModel {
            user_id: row.get("user_id")?,
            event_id: row.get("event_id")?,
            updated_at: row.get("updated_at")?,
        })
    }

    pub async fn get(db: &crate::DatabaseManager, user_id: &str) -> Result<Option<Self>> {
        let user_id = user_id.to_string();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, event_id, updated_at FROM core_event_cursors WHERE user_id = ?1",
            )?;
            match stmt.query_row([&user_id], CoreEventCursorModel::from_row) {
                Ok(entry) => Ok(Some(entry)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(anyhow::Error::from(e)),
            }
        })
        .await?
    }

    pub async fn upsert(db: &crate::DatabaseManager, user_id: &str, event_id: &str) -> Result<()> {
        let user_id = user_id.to_string();
        let event_id = event_id.to_string();
        let updated_at = jiff::Timestamp::now().as_second();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO core_event_cursors (user_id, event_id, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id) DO UPDATE SET
                 event_id = excluded.event_id,
                 updated_at = excluded.updated_at",
                params![user_id, event_id, updated_at],
            )?;
            Ok(())
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::create_test_db;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_returns_none_when_absent() {
        let db = test_db!();
        let result = CoreEventCursorModel::get(&db, "user1").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_and_get() {
        let db = test_db!();
        let before = jiff::Timestamp::now().as_second();
        CoreEventCursorModel::upsert(&db, "user1", "event-abc")
            .await
            .unwrap();
        let entry = CoreEventCursorModel::get(&db, "user1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.event_id, "event-abc");
        assert!(entry.updated_at >= before);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_overwrites() {
        let db = test_db!();
        CoreEventCursorModel::upsert(&db, "user1", "event-abc")
            .await
            .unwrap();
        CoreEventCursorModel::upsert(&db, "user1", "event-xyz")
            .await
            .unwrap();
        let entry = CoreEventCursorModel::get(&db, "user1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.event_id, "event-xyz");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cursors_are_isolated_per_user() {
        let db = test_db!();
        CoreEventCursorModel::upsert(&db, "user1", "event-abc")
            .await
            .unwrap();
        CoreEventCursorModel::upsert(&db, "user2", "event-xyz")
            .await
            .unwrap();
        let entry1 = CoreEventCursorModel::get(&db, "user1")
            .await
            .unwrap()
            .unwrap();
        let entry2 = CoreEventCursorModel::get(&db, "user2")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry1.event_id, "event-abc");
        assert_eq!(entry2.event_id, "event-xyz");
    }
}
