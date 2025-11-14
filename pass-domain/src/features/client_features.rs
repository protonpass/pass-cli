use crate::{AccountCrypto, FsStorage, LocalKeyProvider, PgpCrypto, TelemetryHandler};
use anyhow::Result;
use std::any::Any;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait ClientFeatures: Send + Sync {
    async fn get_local_key_provider(&self) -> Result<Arc<dyn LocalKeyProvider>>;
    async fn get_account_crypto(&self) -> Arc<dyn AccountCrypto>;
    async fn get_fs(&self) -> Arc<dyn FsStorage>;
    async fn get_pgp_crypto(&self) -> Arc<dyn PgpCrypto>;
    async fn get_telemetry_handler(&self) -> Arc<dyn TelemetryHandler>;

    /// Allow downcasting to concrete implementations
    fn as_any(&self) -> &dyn Any;
}
