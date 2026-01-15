use anyhow::Result;
use rusqlite::{Row, params};

#[derive(Debug, Clone)]
pub struct ShareKeyModel {
    pub id: i64,
    pub user_id: String,
    pub share_id: String,
    pub key_rotation: u8,
    pub share_key: Vec<u8>,
    pub created_at: i64,
}

impl ShareKeyModel {
    pub fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(ShareKeyModel {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            share_id: row.get("share_id")?,
            key_rotation: row.get::<_, i64>("key_rotation")? as u8,
            share_key: row.get("share_key")?,
            created_at: row.get("created_at")?,
        })
    }

    pub async fn insert(
        db: &crate::DatabaseManager,
        user_id: &str,
        share_id: &str,
        key_rotation: u8,
        share_key: Vec<u8>,
    ) -> Result<i64> {
        let user_id = user_id.to_string();
        let share_id = share_id.to_string();
        let created_at = jiff::Timestamp::now().as_second();

        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO share_keys (user_id, share_id, key_rotation, share_key, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(user_id, share_id, key_rotation) DO UPDATE SET
                 share_key = excluded.share_key,
                 created_at = excluded.created_at",
                params![
                    user_id,
                    share_id,
                    key_rotation as i64,
                    share_key,
                    created_at
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await?
    }

    pub async fn get_by_share_id(
        db: &crate::DatabaseManager,
        user_id: &str,
        share_id: &str,
    ) -> Result<Vec<ShareKeyModel>> {
        let user_id = user_id.to_string();
        let share_id = share_id.to_string();

        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, share_id, key_rotation, share_key, created_at
                 FROM share_keys
                 WHERE user_id = ?1 AND share_id = ?2
                 ORDER BY key_rotation ASC",
            )?;

            let keys = stmt
                .query_map([&user_id, &share_id], |row| {
                    Ok(ShareKeyModel::from_row(row))
                })?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect::<Result<Vec<ShareKeyModel>>>()?;

            Ok(keys)
        })
        .await?
    }

    pub async fn delete_by_user_id(db: &crate::DatabaseManager, user_id: &str) -> Result<usize> {
        let user_id = user_id.to_string();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let count = conn.execute("DELETE FROM share_keys WHERE user_id = ?1", [&user_id])?;
            Ok(count)
        })
        .await?
    }

    pub async fn delete_by_share_id(
        db: &crate::DatabaseManager,
        user_id: &str,
        share_id: &str,
    ) -> Result<usize> {
        let user_id = user_id.to_string();
        let share_id = share_id.to_string();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let count = conn.execute(
                "DELETE FROM share_keys WHERE user_id = ?1 AND share_id = ?2",
                [&user_id, &share_id],
            )?;
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
    async fn test_insert_and_retrieve_share_keys() {
        let db = test_db!();

        let user_id = "test_user";
        let share_id = "test_share";
        let key_rotation = 1u8;
        let share_key = vec![1, 2, 3, 4, 5];

        let id = ShareKeyModel::insert(&db, user_id, share_id, key_rotation, share_key.clone())
            .await
            .unwrap();

        assert!(id > 0);

        let keys = ShareKeyModel::get_by_share_id(&db, user_id, share_id)
            .await
            .unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].user_id, user_id);
        assert_eq!(keys[0].share_id, share_id);
        assert_eq!(keys[0].key_rotation, key_rotation);
        assert_eq!(keys[0].share_key, share_key);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_multiple_rotations() {
        let db = test_db!();

        let user_id = "test_user";
        let share_id = "test_share";

        // Insert multiple key rotations
        for rotation in 1..=3 {
            let share_key = vec![rotation; 32];
            ShareKeyModel::insert(&db, user_id, share_id, rotation, share_key)
                .await
                .unwrap();
        }

        let keys = ShareKeyModel::get_by_share_id(&db, user_id, share_id)
            .await
            .unwrap();

        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].key_rotation, 1);
        assert_eq!(keys[1].key_rotation, 2);
        assert_eq!(keys[2].key_rotation, 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_on_conflict() {
        let db = test_db!();

        let user_id = "test_user";
        let share_id = "test_share";
        let key_rotation = 1u8;
        let share_key_v1 = vec![1, 2, 3];
        let share_key_v2 = vec![4, 5, 6];

        // Insert first version
        ShareKeyModel::insert(&db, user_id, share_id, key_rotation, share_key_v1)
            .await
            .unwrap();

        // Insert second version with same key_rotation (should update)
        ShareKeyModel::insert(&db, user_id, share_id, key_rotation, share_key_v2.clone())
            .await
            .unwrap();

        let keys = ShareKeyModel::get_by_share_id(&db, user_id, share_id)
            .await
            .unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].share_key, share_key_v2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id() {
        let db = test_db!();

        let user1 = "user1";
        let user2 = "user2";
        let share_id = "test_share";

        // Insert keys for both users
        ShareKeyModel::insert(&db, user1, share_id, 1, vec![1, 2, 3])
            .await
            .unwrap();
        ShareKeyModel::insert(&db, user2, share_id, 1, vec![4, 5, 6])
            .await
            .unwrap();

        // Delete user1's keys
        let deleted = ShareKeyModel::delete_by_user_id(&db, user1).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify user1's keys are gone
        let user1_keys = ShareKeyModel::get_by_share_id(&db, user1, share_id)
            .await
            .unwrap();
        assert_eq!(user1_keys.len(), 0);

        // Verify user2's keys remain
        let user2_keys = ShareKeyModel::get_by_share_id(&db, user2, share_id)
            .await
            .unwrap();
        assert_eq!(user2_keys.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_share_id() {
        let db = test_db!();

        let user_id = "test_user";
        let share1 = "share1";
        let share2 = "share2";

        // Insert keys for both shares
        ShareKeyModel::insert(&db, user_id, share1, 1, vec![1, 2, 3])
            .await
            .unwrap();
        ShareKeyModel::insert(&db, user_id, share2, 1, vec![4, 5, 6])
            .await
            .unwrap();

        // Delete share1's keys
        let deleted = ShareKeyModel::delete_by_share_id(&db, user_id, share1)
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify share1's keys are gone
        let share1_keys = ShareKeyModel::get_by_share_id(&db, user_id, share1)
            .await
            .unwrap();
        assert_eq!(share1_keys.len(), 0);

        // Verify share2's keys remain
        let share2_keys = ShareKeyModel::get_by_share_id(&db, user_id, share2)
            .await
            .unwrap();
        assert_eq!(share2_keys.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_nonexistent_share() {
        let db = test_db!();

        let keys = ShareKeyModel::get_by_share_id(&db, "nonexistent_user", "nonexistent_share")
            .await
            .unwrap();

        assert_eq!(keys.len(), 0);
    }
}
