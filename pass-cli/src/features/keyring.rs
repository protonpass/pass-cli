use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};
use pass_domain::LocalKeyProvider;
use tokio::sync::RwLock;

const KEYRING_SERVICE_NAME: &str = "ProtonPassCLI";
const KEYRING_CREDENTIAL_NAME: &str = "cli-local-key";

pub struct KeyringKeyProvider {
    key: RwLock<Option<Vec<u8>>>,
    xor_key: u8,
}

impl KeyringKeyProvider {
    pub fn new() -> Self {
        Self {
            key: RwLock::new(None),
            xor_key: pass_domain::crypto::generate_random_byte(),
        }
    }

    fn build_entry() -> Result<Entry> {
        let entry = Entry::new(KEYRING_SERVICE_NAME, KEYRING_CREDENTIAL_NAME)
            .map_err(|e| anyhow::anyhow!("Error accessing credential: {e:?}"))?;

        Ok(entry)
    }

    async fn get_local_key() -> Result<Vec<u8>> {
        let entry = Self::build_entry()?;

        match entry.get_secret() {
            Ok(cred) => Ok(cred),
            Err(e) => match e {
                KeyringError::NoEntry => {
                    info!("Credential not found in Keyring. Creating one");
                    let cred = pass_domain::crypto::generate_encryption_key();
                    entry
                        .set_secret(&cred)
                        .map_err(|e| anyhow::anyhow!("Error accessing keyring: {e}"))?;
                    info!("Stored credential into keyring");
                    Ok(cred)
                }
                _ => Err(anyhow::anyhow!(
                    "Error accessing credential on keyring: {e:?}"
                )),
            },
        }
    }

    fn xor_key(&self, key: &[u8]) -> Vec<u8> {
        let mut res = Vec::with_capacity(key.len());
        for b in key {
            res.push(self.xor_key ^ b);
        }
        res
    }
}

#[async_trait::async_trait]
impl LocalKeyProvider for KeyringKeyProvider {
    async fn get_key(&self) -> Result<Vec<u8>> {
        let key_guard = self.key.read().await;
        if let Some(key) = &*key_guard {
            Ok(self.xor_key(key))
        } else {
            drop(key_guard);
            let mut write_key_guard = self.key.write().await;
            let key = Self::get_local_key()
                .await
                .context("Could not get local key from keyring")?;

            let xored_key = self.xor_key(&key);
            *write_key_guard = Some(xored_key);
            Ok(key)
        }
    }
}
