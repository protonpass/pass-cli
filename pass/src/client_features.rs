use crate::account::{Passphrase, UnlockedAddressKeys};
use crate::{ApiKey, ApiKeySalt, PgpCrypto, UserKey};
use pass_domain::AddressKey;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait ClientFeatures: Send + Sync {
    async fn get_local_key(&self) -> anyhow::Result<Vec<u8>>;
    async fn get_file(&self, path: &Path) -> anyhow::Result<Vec<u8>>;
    async fn file_exists(&self, path: &Path) -> anyhow::Result<bool>;
    async fn store_file(&self, contents: Vec<u8>, path: &Path) -> anyhow::Result<()>;
    async fn remove_file(&self, path: &Path) -> anyhow::Result<()>;

    async fn generate_passphrases(
        &self,
        key_salts: Vec<ApiKeySalt>,
        pass: &str,
    ) -> anyhow::Result<HashMap<String, Vec<u8>>>;
    async fn open_user_keys(
        &self,
        keys: Vec<ApiKey>,
        passphrases: HashMap<String, Passphrase>,
    ) -> anyhow::Result<Vec<UserKey>>;

    async fn open_address_keys(
        &self,
        user_keys: Vec<UserKey>,
        address_keys: Vec<AddressKey>,
    ) -> anyhow::Result<UnlockedAddressKeys>;

    async fn get_pgp_crypto(&self) -> Arc<dyn PgpCrypto>;
}
