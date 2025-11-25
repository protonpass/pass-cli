use anyhow::Result;
use pass_db::{DatabaseManager, ShareKeyModel};
use pass_domain::{DecryptedShareKey, ShareId, ShareKeyStorage};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DatabaseShareKeyStorage {
    db: DatabaseManager,
    user_id: Arc<RwLock<Option<String>>>,
}

impl DatabaseShareKeyStorage {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            db,
            user_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_user_id(&self, user_id: Option<String>) {
        *self.user_id.write().await = user_id;
    }
}

#[async_trait::async_trait]
impl ShareKeyStorage for DatabaseShareKeyStorage {
    async fn get_share_keys(&self, share_id: &ShareId) -> Result<Option<Vec<DecryptedShareKey>>> {
        let user_id = self.user_id.read().await.clone();

        let user_id = match user_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let models = ShareKeyModel::get_by_share_id(&self.db, &user_id, share_id.value()).await?;

        if models.is_empty() {
            return Ok(None);
        }

        let keys = models
            .into_iter()
            .map(|model| DecryptedShareKey::new(model.key_rotation, model.share_key))
            .collect();

        Ok(Some(keys))
    }

    async fn store_share_keys(
        &self,
        share_id: &ShareId,
        share_keys: Vec<DecryptedShareKey>,
    ) -> Result<()> {
        let user_id = self.user_id.read().await.clone();

        let user_id = match user_id {
            Some(id) => id,
            None => {
                warn!("No user_id set, skipping share key storage");
                return Ok(());
            }
        };

        for key in share_keys {
            ShareKeyModel::insert(
                &self.db,
                &user_id,
                share_id.value(),
                key.key_rotation,
                key.key().to_vec(),
            )
            .await?;
        }

        Ok(())
    }
}
