use anyhow::Result;
use async_lock::RwLock;
use pass_domain::{
    AccountCrypto, ClientFeatures, DataStorage, DecryptedShareKey, FsStorage, LocalKey,
    LocalKeyProvider, PgpCrypto, ShareId, ShareKeyStorage,
};
use pass_fs::InMemoryFsStorage;
use pass_pgp::{NativePgpCrypto, ProtonAccountCrypto};
use std::collections::HashMap;
use std::sync::Arc;

pub struct StaticKeyProvider {
    pub key: Vec<u8>,
}

#[async_trait::async_trait]
impl LocalKeyProvider for StaticKeyProvider {
    async fn get_key(&self) -> Result<LocalKey> {
        Ok(LocalKey::new(self.key.clone()))
    }
    async fn remove_key(&self) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryShareKeyStorage {
    storage: Arc<RwLock<HashMap<ShareId, Vec<DecryptedShareKey>>>>,
}

impl InMemoryShareKeyStorage {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl ShareKeyStorage for InMemoryShareKeyStorage {
    async fn get_share_keys(&self, share_id: &ShareId) -> Result<Option<Vec<DecryptedShareKey>>> {
        let storage = self.storage.read().await;
        Ok(storage.get(share_id).cloned())
    }

    async fn store_share_keys(
        &self,
        share_id: &ShareId,
        share_keys: Vec<DecryptedShareKey>,
    ) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert(share_id.clone(), share_keys);
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryDataStorage {
    share_key_storage: Arc<dyn ShareKeyStorage>,
}

impl InMemoryDataStorage {
    pub fn new() -> Self {
        Self {
            share_key_storage: Arc::new(InMemoryShareKeyStorage::new()),
        }
    }
}

#[async_trait::async_trait]
impl DataStorage for InMemoryDataStorage {
    async fn get_share_key_storage(&self) -> Arc<dyn ShareKeyStorage> {
        self.share_key_storage.clone()
    }
}

#[derive(Clone)]
pub struct TestClientFeatures {
    pub storage: Arc<InMemoryFsStorage>,
    pub key_provider: Arc<StaticKeyProvider>,
    pub data_storage: Arc<dyn DataStorage>,
}

impl TestClientFeatures {
    pub fn new(key: Vec<u8>) -> Self {
        Self {
            storage: Arc::new(InMemoryFsStorage::new()),
            key_provider: Arc::new(StaticKeyProvider { key }),
            data_storage: Arc::new(InMemoryDataStorage::new()),
        }
    }
}

#[async_trait::async_trait]
impl ClientFeatures for TestClientFeatures {
    async fn get_local_key_provider(&self) -> Result<Arc<dyn LocalKeyProvider>> {
        Ok(self.key_provider.clone())
    }

    async fn get_account_crypto(&self) -> Arc<dyn AccountCrypto> {
        Arc::new(ProtonAccountCrypto)
    }

    async fn get_fs(&self) -> Arc<dyn FsStorage> {
        self.storage.clone()
    }

    async fn get_pgp_crypto(&self) -> Arc<dyn PgpCrypto> {
        Arc::new(NativePgpCrypto)
    }

    async fn get_telemetry_handler(&self) -> Arc<dyn pass_domain::TelemetryHandler> {
        Arc::new(pass_domain::NoopTelemetryHandler)
    }

    async fn get_data_storage(&self) -> Result<Arc<dyn DataStorage>> {
        Ok(self.data_storage.clone())
    }
}
