use crate::DbConnection;
use anyhow::Result;
use rusqlite::{OptionalExtension, Row, params};

#[derive(Debug, Clone, PartialEq)]
pub struct UserSettingModel {
    pub user_id: String,
    pub setting_key: String,
    pub setting_value: Option<String>,
    pub updated_at: i64,
}

impl UserSettingModel {
    pub fn from_row(row: &Row<'_>) -> Result<Self> {
        Ok(UserSettingModel {
            user_id: row.get("user_id")?,
            setting_key: row.get("setting_key")?,
            setting_value: row.get("setting_value")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// Set or update a setting (INSERT OR REPLACE)
    pub async fn upsert(
        conn: &DbConnection,
        user_id: &str,
        setting_key: &str,
        setting_value: Option<String>,
    ) -> Result<()> {
        let user_id = user_id.to_string();
        let setting_key = setting_key.to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO user_settings (user_id, setting_key, setting_value, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![user_id, setting_key, setting_value, timestamp],
            )?;
            Ok(())
        })
        .await?
    }

    /// Get a specific setting for a user
    pub async fn get(
        conn: &DbConnection,
        user_id: &str,
        setting_key: &str,
    ) -> Result<Option<UserSettingModel>> {
        let user_id = user_id.to_string();
        let setting_key = setting_key.to_string();

        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, setting_key, setting_value, updated_at FROM user_settings WHERE user_id = ?1 AND setting_key = ?2",
            )?;

            let result = stmt
                .query_row(params![user_id, setting_key], |row| {
                    Ok(UserSettingModel::from_row(row))
                })
                .optional()?
                .transpose()?;

            Ok(result)
        })
        .await?
    }

    /// Get all settings for a user
    pub async fn get_by_user_id(
        conn: &DbConnection,
        user_id: &str,
    ) -> Result<Vec<UserSettingModel>> {
        let user_id = user_id.to_string();

        conn.interact(move |conn| {
            let mut stmt = conn.prepare(
                "SELECT user_id, setting_key, setting_value, updated_at FROM user_settings WHERE user_id = ?1 ORDER BY setting_key ASC",
            )?;

            let records = stmt
                .query_map([user_id], |row| Ok(UserSettingModel::from_row(row)))?
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .collect::<Result<Vec<_>>>()?;

            Ok(records)
        })
        .await?
    }

    /// Delete a specific setting (used by unset command)
    pub async fn delete(conn: &DbConnection, user_id: &str, setting_key: &str) -> Result<usize> {
        let user_id = user_id.to_string();
        let setting_key = setting_key.to_string();

        conn.interact(move |conn| {
            let count = conn.execute(
                "DELETE FROM user_settings WHERE user_id = ?1 AND setting_key = ?2",
                params![user_id, setting_key],
            )?;
            Ok(count)
        })
        .await?
    }

    /// Delete all settings for a user
    pub async fn delete_by_user_id(conn: &DbConnection, user_id: &str) -> Result<usize> {
        let user_id = user_id.to_string();

        conn.interact(move |conn| {
            let count = conn.execute("DELETE FROM user_settings WHERE user_id = ?1", [user_id])?;
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

        let user_id = "user123";
        let setting_key = "default_share_id";
        let setting_value = Some("vault-id-123".to_string());

        UserSettingModel::upsert(&conn, user_id, setting_key, setting_value.clone())
            .await
            .unwrap();

        let retrieved = UserSettingModel::get(&conn, user_id, setting_key)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.setting_key, setting_key);
        assert_eq!(retrieved.setting_value, setting_value);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_upsert_updates_existing() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = "user456";
        let setting_key = "default_format";
        let value1 = Some("human".to_string());
        let value2 = Some("json".to_string());

        // Insert first time
        UserSettingModel::upsert(&conn, user_id, setting_key, value1)
            .await
            .unwrap();

        let retrieved = UserSettingModel::get(&conn, user_id, setting_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.setting_value, Some("human".to_string()));

        // Update with new value
        UserSettingModel::upsert(&conn, user_id, setting_key, value2)
            .await
            .unwrap();

        let retrieved = UserSettingModel::get(&conn, user_id, setting_key)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.setting_value, Some("json".to_string()));

        // Verify only one record exists
        let all_records = UserSettingModel::get_by_user_id(&conn, user_id)
            .await
            .unwrap();
        assert_eq!(all_records.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_nonexistent() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let result = UserSettingModel::get(&conn, "nonexistent", "setting")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user1 = "user1";
        let user2 = "user2";

        // Insert multiple settings for user1
        UserSettingModel::upsert(&conn, user1, "default_share_id", Some("vault1".to_string()))
            .await
            .unwrap();
        UserSettingModel::upsert(&conn, user1, "default_format", Some("human".to_string()))
            .await
            .unwrap();

        // Insert setting for user2
        UserSettingModel::upsert(&conn, user2, "default_share_id", Some("vault2".to_string()))
            .await
            .unwrap();

        let user1_records = UserSettingModel::get_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(user1_records.len(), 2);
        assert!(user1_records.iter().all(|r| r.user_id.as_str() == "user1"));

        let user2_records = UserSettingModel::get_by_user_id(&conn, user2)
            .await
            .unwrap();
        assert_eq!(user2_records.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = "user789";
        let setting_key = "test_setting";

        UserSettingModel::upsert(&conn, user_id, setting_key, Some("value".to_string()))
            .await
            .unwrap();

        let retrieved = UserSettingModel::get(&conn, user_id, setting_key)
            .await
            .unwrap();
        assert!(retrieved.is_some());

        let deleted_count = UserSettingModel::delete(&conn, user_id, setting_key)
            .await
            .unwrap();
        assert_eq!(deleted_count, 1);

        let retrieved = UserSettingModel::get(&conn, user_id, setting_key)
            .await
            .unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete_by_user_id() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user1 = "user1";
        let user2 = "user2";

        // Insert multiple settings for user1
        UserSettingModel::upsert(&conn, user1, "setting1", Some("value1".to_string()))
            .await
            .unwrap();
        UserSettingModel::upsert(&conn, user1, "setting2", Some("value2".to_string()))
            .await
            .unwrap();

        // Insert setting for user2
        UserSettingModel::upsert(&conn, user2, "setting3", Some("value3".to_string()))
            .await
            .unwrap();

        let deleted_count = UserSettingModel::delete_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(deleted_count, 2);

        let user1_records = UserSettingModel::get_by_user_id(&conn, user1)
            .await
            .unwrap();
        assert_eq!(user1_records.len(), 0);

        let user2_records = UserSettingModel::get_by_user_id(&conn, user2)
            .await
            .unwrap();
        assert_eq!(user2_records.len(), 1); // user2 remains
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_null_setting_value() {
        let db = test_db!();
        let conn = db.get_connection().await.unwrap();

        let user_id = "user_null";
        let setting_key = "nullable_setting";

        UserSettingModel::upsert(&conn, user_id, setting_key, None)
            .await
            .unwrap();

        let retrieved = UserSettingModel::get(&conn, user_id, setting_key)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(retrieved.user_id, user_id);
        assert_eq!(retrieved.setting_key, setting_key);
        assert_eq!(retrieved.setting_value, None);
    }
}
