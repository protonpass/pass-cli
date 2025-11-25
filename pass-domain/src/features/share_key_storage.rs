use crate::{DecryptedShareKey, ShareId};
use anyhow::Result;

#[async_trait::async_trait]
pub trait ShareKeyStorage: Send + Sync {
    async fn get_share_keys(&self, share_id: &ShareId) -> Result<Option<Vec<DecryptedShareKey>>>;
    async fn store_share_keys(
        &self,
        share_id: &ShareId,
        share_keys: Vec<DecryptedShareKey>,
    ) -> Result<()>;
}
