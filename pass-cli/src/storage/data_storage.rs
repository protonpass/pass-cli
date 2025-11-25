use pass_domain::{DataStorage, ShareKeyStorage};
use std::sync::Arc;

pub struct CliDataStorage {
    share_key_storage: Arc<dyn ShareKeyStorage>,
}

impl CliDataStorage {
    pub fn new(share_key_storage: Arc<dyn ShareKeyStorage>) -> Self {
        Self { share_key_storage }
    }
}

#[async_trait::async_trait]
impl DataStorage for CliDataStorage {
    async fn get_share_key_storage(&self) -> Arc<dyn ShareKeyStorage> {
        self.share_key_storage.clone()
    }
}
