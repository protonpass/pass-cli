use anyhow::Result;
use rusqlite::{Row, params};

#[derive(Debug, Clone)]
pub struct FolderKeyModel {
    pub id: i64,
    pub user_id: String,
    pub share_id: String,
    pub folder_id: String,
    pub key_rotation: u8,
    pub folder_key: Vec<u8>,
    pub created_at: i64,
}

impl FolderKeyModel {
    pub fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(FolderKeyModel {
            id: row.get("id")?,
            user_id: row.get("user_id")?,
            share_id: row.get("share_id")?,
            folder_id: row.get("folder_id")?,
            key_rotation: row.get::<_, i64>("key_rotation")? as u8,
            folder_key: row.get("folder_key")?,
            created_at: row.get("created_at")?,
        })
    }

    pub async fn insert(
        db: &crate::DatabaseManager,
        user_id: &str,
        share_id: &str,
        folder_id: &str,
        key_rotation: u8,
        folder_key: Vec<u8>,
    ) -> Result<i64> {
        let user_id = user_id.to_string();
        let share_id = share_id.to_string();
        let folder_id = folder_id.to_string();
        let created_at = chrono::Utc::now().timestamp();

        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT INTO folder_keys (user_id, share_id, folder_id, key_rotation, folder_key, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(user_id, share_id, folder_id, key_rotation) DO UPDATE SET
                 folder_key = excluded.folder_key,
                 created_at = excluded.created_at",
                params![
                    user_id,
                    share_id,
                    folder_id,
                    key_rotation as i64,
                    folder_key,
                    created_at
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await?
    }

    pub async fn get_by_folder_id(
        db: &crate::DatabaseManager,
        user_id: &str,
        share_id: &str,
        folder_id: &str,
    ) -> Result<Vec<FolderKeyModel>> {
        let user_id = user_id.to_string();
        let share_id = share_id.to_string();
        let folder_id = folder_id.to_string();

        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, share_id, folder_id, key_rotation, folder_key, created_at
                 FROM folder_keys
                 WHERE user_id = ?1 AND share_id = ?2 AND folder_id = ?3
                 ORDER BY key_rotation ASC",
            )?;

            let keys = stmt
                .query_map([&user_id, &share_id, &folder_id], |row| {
                    Ok(FolderKeyModel::from_row(row))
                })?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect::<Result<Vec<FolderKeyModel>>>()?;

            Ok(keys)
        })
        .await?
    }

    pub async fn delete_by_user_id(db: &crate::DatabaseManager, user_id: &str) -> Result<usize> {
        let user_id = user_id.to_string();
        let conn = db.get_connection().await?;
        conn.interact(move |conn| {
            let count = conn.execute("DELETE FROM folder_keys WHERE user_id = ?1", [&user_id])?;
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
                "DELETE FROM folder_keys WHERE user_id = ?1 AND share_id = ?2",
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
    async fn test_insert_and_retrieve_folder_keys() {
        let db = test_db!();

        let user_id = "test_user";
        let share_id = "test_share";
        let folder_id = "test_folder";
        let key_rotation = 1u8;
        let folder_key = vec![1, 2, 3, 4, 5];

        let id = FolderKeyModel::insert(
            &db,
            user_id,
            share_id,
            folder_id,
            key_rotation,
            folder_key.clone(),
        )
        .await
        .unwrap();

        assert!(id > 0);

        let keys = FolderKeyModel::get_by_folder_id(&db, user_id, share_id, folder_id)
            .await
            .unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].user_id, user_id);
        assert_eq!(keys[0].share_id, share_id);
        assert_eq!(keys[0].folder_id, folder_id);
        assert_eq!(keys[0].key_rotation, key_rotation);
        assert_eq!(keys[0].folder_key, folder_key);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_insert_multiple_rotations() {
        let db = test_db!();

        let user_id = "test_user";
        let share_id = "test_share";
        let folder_id = "test_folder";

        // Insert multiple key rotations
        for rotation in 1..=3 {
            let folder_key = vec![rotation; 32];
            FolderKeyModel::insert(&db, user_id, share_id, folder_id, rotation, folder_key)
                .await
                .unwrap();
        }

        let keys = FolderKeyModel::get_by_folder_id(&db, user_id, share_id, folder_id)
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
        let folder_id = "test_folder";
        let key_rotation = 1u8;
        let folder_key_v1 = vec![1, 2, 3];
        let folder_key_v2 = vec![4, 5, 6];

        // Insert first version
        FolderKeyModel::insert(
            &db,
            user_id,
            share_id,
            folder_id,
            key_rotation,
            folder_key_v1,
        )
        .await
        .unwrap();

        // Insert second version with same key_rotation (should update)
        FolderKeyModel::insert(
            &db,
            user_id,
            share_id,
            folder_id,
            key_rotation,
            folder_key_v2.clone(),
        )
        .await
        .unwrap();

        let keys = FolderKeyModel::get_by_folder_id(&db, user_id, share_id, folder_id)
            .await
            .unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].folder_key, folder_key_v2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id() {
        let db = test_db!();

        let user1 = "user1";
        let user2 = "user2";
        let share_id = "test_share";
        let folder_id = "test_folder";

        // Insert keys for both users
        FolderKeyModel::insert(&db, user1, share_id, folder_id, 1, vec![1, 2, 3])
            .await
            .unwrap();
        FolderKeyModel::insert(&db, user2, share_id, folder_id, 1, vec![4, 5, 6])
            .await
            .unwrap();

        // Delete user1's keys
        let deleted = FolderKeyModel::delete_by_user_id(&db, user1).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify user1's keys are gone
        let user1_keys = FolderKeyModel::get_by_folder_id(&db, user1, share_id, folder_id)
            .await
            .unwrap();
        assert_eq!(user1_keys.len(), 0);

        // Verify user2's keys remain
        let user2_keys = FolderKeyModel::get_by_folder_id(&db, user2, share_id, folder_id)
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
        let folder_id = "test_folder";

        // Insert keys for both shares
        FolderKeyModel::insert(&db, user_id, share1, folder_id, 1, vec![1, 2, 3])
            .await
            .unwrap();
        FolderKeyModel::insert(&db, user_id, share2, folder_id, 1, vec![4, 5, 6])
            .await
            .unwrap();

        // Delete share1's keys
        let deleted = FolderKeyModel::delete_by_share_id(&db, user_id, share1)
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        // Verify share1's keys are gone
        let share1_keys = FolderKeyModel::get_by_folder_id(&db, user_id, share1, folder_id)
            .await
            .unwrap();
        assert_eq!(share1_keys.len(), 0);

        // Verify share2's keys remain
        let share2_keys = FolderKeyModel::get_by_folder_id(&db, user_id, share2, folder_id)
            .await
            .unwrap();
        assert_eq!(share2_keys.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_nonexistent_folder() {
        let db = test_db!();

        let keys = FolderKeyModel::get_by_folder_id(
            &db,
            "nonexistent_user",
            "nonexistent_share",
            "nonexistent_folder",
        )
        .await
        .unwrap();

        assert_eq!(keys.len(), 0);
    }
}
