use crate::constants::SESSION_FILE_NAME;
use anyhow::{Context, Result};
use keyring::{Entry, Error as KeyringError};
use pass_domain::utils::xor_key_multibyte;
use pass_domain::{LocalKey, LocalKeyProvider};
use std::path::PathBuf;
use tokio::sync::RwLock;

const KEYRING_SERVICE_NAME: &str = "ProtonPassCLI";
const KEYRING_CREDENTIAL_NAME: &str = "cli-local-key";
const XOR_KEY_LENGTH: usize = 32;

pub struct KeyringKeyProvider {
    key: RwLock<Option<Vec<u8>>>,
    xor_key: Vec<u8>,
    base_dir: PathBuf,
}

impl KeyringKeyProvider {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let xor_key = pass_domain::crypto::random_bytes(XOR_KEY_LENGTH);
        Ok(Self {
            key: RwLock::new(None),
            xor_key,
            base_dir,
        })
    }

    fn session_exists(&self) -> bool {
        let session_path = self.base_dir.join(SESSION_FILE_NAME);
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
                        eprintln!("Run 'pass-cli login' to authenticate again.");

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
                KeyringError::PlatformFailure(ref err) => {
                    let as_str = err.to_string();
                    if as_str.contains("User canceled the operation") {
                        eprintln!("Please authorize access to the system keyring");
                        std::process::exit(1);
                    } else {
                        Err(anyhow::anyhow!(
                            "Error accessing credential on keyring: {e:?}"
                        ))
                    }
                }
                _ => Err(anyhow::anyhow!(
                    "Error accessing credential on keyring: {e:?}"
                )),
            },
        }
    }
}

#[async_trait::async_trait]
impl LocalKeyProvider for KeyringKeyProvider {
    async fn get_key(&self) -> Result<LocalKey> {
        let key_guard = self.key.read().await;
        if let Some(key) = &*key_guard {
            Ok(LocalKey::new(xor_key_multibyte(key, &self.xor_key)))
        } else {
            drop(key_guard);
            let mut write_key_guard = self.key.write().await;
            if let Some(key) = &*write_key_guard {
                return Ok(LocalKey::new(xor_key_multibyte(key, &self.xor_key)));
            }

            let key = self
                .get_local_key()
                .await
                .context("Could not get local key from keyring")?;

            let xored_key = xor_key_multibyte(&key, &self.xor_key);
            *write_key_guard = Some(xored_key);
            Ok(LocalKey::new(key))
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
