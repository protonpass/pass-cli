use anyhow::Result;
use pass_domain::{AccountCrypto, ClientFeatures, FsStorage, LocalKeyProvider, PgpCrypto};
use pass_fs::InMemoryFsStorage;
use pass_pgp::{NativePgpCrypto, ProtonAccountCrypto};
use std::sync::Arc;

pub struct StaticKeyProvider {
    pub key: Vec<u8>,
}

#[async_trait::async_trait]
impl LocalKeyProvider for StaticKeyProvider {
    async fn get_key(&self) -> Result<Vec<u8>> {
        Ok(self.key.clone())
    }
}

#[derive(Clone)]
pub struct TestClientFeatures {
    pub storage: Arc<InMemoryFsStorage>,
    pub key_provider: Arc<StaticKeyProvider>,
}

impl TestClientFeatures {
    pub fn new(key: Vec<u8>) -> Self {
        Self {
            storage: Arc::new(InMemoryFsStorage::new()),
            key_provider: Arc::new(StaticKeyProvider { key }),
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
}
