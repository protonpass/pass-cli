use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};
use pass_domain::LocalKeyProvider;
use std::path::PathBuf;
use tokio::sync::RwLock;

const KEYRING_SERVICE_NAME: &str = "ProtonPassCLI";
const KEYRING_CREDENTIAL_NAME: &str = "cli-local-key";

pub struct KeyringKeyProvider {
    key: RwLock<Option<Vec<u8>>>,
    xor_key: u8,
    base_dir: PathBuf,
}

impl KeyringKeyProvider {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            key: RwLock::new(None),
            xor_key: pass_domain::crypto::generate_random_byte(),
            base_dir,
        })
    }

    fn session_exists(&self) -> bool {
        let session_path = self.base_dir.join(crate::store::FILE_NAME);
        session_path.exists() && session_path.is_file()
    }

    fn build_entry() -> Result<Entry> {
        let entry = Entry::new(KEYRING_SERVICE_NAME, KEYRING_CREDENTIAL_NAME)
            .map_err(|e| anyhow::anyhow!("Error accessing credential: {e:?}"))?;

        Ok(entry)
    }

    async fn get_local_key(&self) -> Result<Vec<u8>> {
        let entry = Self::build_entry()?;

        match entry.get_secret() {
            Ok(cred) => Ok(cred),
            Err(e) => match e {
                KeyringError::NoEntry => {
                    // Check if session exists - if so, log the user out
                    if self.session_exists() {
                        eprintln!(
                            "Error: Local encryption key not found but session exists. Forcing logout for security."
                        );

                        // Perform force logout
                        if let Err(logout_err) = crate::commands::logout::force_logout().await {
                            error!("Error during force logout: {logout_err:#}");
                        }
                        eprintln!("Run 'protonpass login' to authenticate again.");

                        // Finish the process
                        std::process::exit(1);
                    } else {
                        info!("Credential not found in Keyring. Creating one");
                        let cred = pass_domain::crypto::generate_encryption_key();
                        entry
                            .set_secret(&cred)
                            .map_err(|e| anyhow::anyhow!("Error accessing keyring: {e}"))?;
                        info!("Stored credential into keyring");
                        Ok(cred)
                    }
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
            let key = self
                .get_local_key()
                .await
                .context("Could not get local key from keyring")?;

            let xored_key = self.xor_key(&key);
            *write_key_guard = Some(xored_key);
            Ok(key)
        }
    }

    async fn remove_key(&self) -> Result<()> {
        let entry = Self::build_entry()?;
        if let Err(e) = entry.delete_credential() {
            return if let KeyringError::NoEntry = e {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Error deleting credential: {e:?}"))
            };
        }

        Ok(())
    }
}
