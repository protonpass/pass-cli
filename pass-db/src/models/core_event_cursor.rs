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
use rusqlite::params;

pub struct CoreEventCursorModel;

impl CoreEventCursorModel {
    pub async fn get(db: &crate::DatabaseManager, user_id: &str) -> Result<Option<String>> {
        let user_id = user_id.to_string();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare("SELECT cursor FROM core_event_cursors WHERE user_id = ?1")?;
            match stmt.query_row([&user_id], |row| row.get::<_, String>(0)) {
                Ok(cursor) => Ok(Some(cursor)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(anyhow::Error::from(e)),
            }
        })
        .await?
    }

    pub async fn upsert(db: &crate::DatabaseManager, user_id: &str, cursor: &str) -> Result<()> {
        let user_id = user_id.to_string();
        let cursor = cursor.to_string();
        let updated_at = jiff::Timestamp::now().as_second();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO core_event_cursors (user_id, cursor, updated_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(user_id) DO UPDATE SET
                 cursor = excluded.cursor,
                 updated_at = excluded.updated_at",
                params![user_id, cursor, updated_at],
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
        CoreEventCursorModel::upsert(&db, "user1", "event-abc")
            .await
            .unwrap();
        let cursor = CoreEventCursorModel::get(&db, "user1").await.unwrap();
        assert_eq!(cursor, Some("event-abc".to_string()));
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
        let cursor = CoreEventCursorModel::get(&db, "user1").await.unwrap();
        assert_eq!(cursor, Some("event-xyz".to_string()));
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
        assert_eq!(
            CoreEventCursorModel::get(&db, "user1").await.unwrap(),
            Some("event-abc".to_string())
        );
        assert_eq!(
            CoreEventCursorModel::get(&db, "user2").await.unwrap(),
            Some("event-xyz".to_string())
        );
    }
}
