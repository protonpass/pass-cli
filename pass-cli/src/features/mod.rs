mod account;
mod pgp;

use crate::features::account::AccountCrypto;
use crate::features::pgp::NativePgpCrypto;
use crate::storage::get_local_key;
use anyhow::{Context, Result};
use pass::{ApiKey, ApiKeySalt, ClientFeatures, Passphrase, UnlockedAddressKeys, UserKey};
use pass_domain::AddressKey;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct CliClientFeatures {
    pub base_dir: PathBuf,
}

impl CliClientFeatures {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[async_trait::async_trait]
impl ClientFeatures for CliClientFeatures {
    async fn get_local_key(&self) -> Result<Vec<u8>> {
        get_local_key(&self.base_dir).await
    }

    async fn get_file(&self, path: &Path) -> Result<Vec<u8>> {
        tokio::fs::read(self.base_dir.join(path))
            .await
            .context("Error reading file")
    }

    async fn file_exists(&self, path: &Path) -> Result<bool> {
        match tokio::fs::metadata(self.base_dir.join(path)).await {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn store_file(&self, contents: Vec<u8>, path: &Path) -> Result<()> {
        tokio::fs::write(self.base_dir.join(path), &contents)
            .await
            .context("Error writing file")?;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        tokio::fs::remove_file(self.base_dir.join(path))
            .await
            .context("Error deleting file")
    }

    async fn generate_passphrases(
        &self,
        key_salts: Vec<ApiKeySalt>,
        pass: &str,
    ) -> Result<HashMap<String, Vec<u8>>> {
        AccountCrypto.generate_passphrases(key_salts, pass)
    }

    async fn open_user_keys(
        &self,
        keys: Vec<ApiKey>,
        passphrases: HashMap<String, Passphrase>,
    ) -> Result<Vec<UserKey>> {
        AccountCrypto.open_user_keys(keys, passphrases)
    }

    async fn open_address_keys(
        &self,
        user_keys: Vec<UserKey>,
        address_keys: Vec<AddressKey>,
    ) -> Result<UnlockedAddressKeys> {
        AccountCrypto.open_address_keys(user_keys, address_keys)
    }

    async fn get_pgp_crypto(&self) -> Arc<dyn pass::PgpCrypto> {
        Arc::new(NativePgpCrypto)
    }
}
