use crate::constants::SESSION_FILE_NAME;
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use keyring::{Entry, Error as KeyringError};
use pass_domain::utils::xor_key_multibyte;
use pass_domain::{LocalKey, LocalKeyProvider};
use sha2::{Digest, Sha256};
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

    // Creates a session-scoped credential name: `cli-local-key:{sha256(abs(base_dir))}`.
    fn credential_name(&self) -> String {
        let canonical = self
            .base_dir
            .canonicalize()
            .unwrap_or_else(|_| self.base_dir.clone());
        let hash = Sha256::digest(canonical.to_string_lossy().as_bytes());
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        format!("{KEYRING_CREDENTIAL_NAME}:{hex}")
    }

    fn build_entry_named(name: &str) -> Result<Entry> {
        Entry::new(KEYRING_SERVICE_NAME, name)
            .map_err(|e| anyhow::anyhow!("Error accessing credential [name={name}]: {e:?}"))
    }

    fn build_entry(&self) -> Result<Entry> {
        Self::build_entry_named(&self.credential_name())
    }

    fn build_legacy_entry() -> Result<Entry> {
        Self::build_entry_named(KEYRING_CREDENTIAL_NAME)
    }

    // Returns a short fingerprint of `key` for logging: `{first4}…{last4}` of its SHA-256 hex.
    fn key_fingerprint(key: &[u8]) -> String {
        let hash: String = Sha256::digest(key)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        format!("{}...{}", &hash[..4], &hash[hash.len() - 4..])
    }

    fn decode_key(raw: Vec<u8>) -> Vec<u8> {
        if let Ok(s) = std::str::from_utf8(&raw)
            && let Ok(decoded) = URL_SAFE_NO_PAD.decode(s.trim())
        {
            debug!(
                "Key decoded from base64 (fingerprint: {})",
                Self::key_fingerprint(&decoded)
            );
            return decoded;
        }

        // If `raw` is not valid base64, returns it as-is, for backwards compatibility with keys
        // stored as raw bytes.
        debug!(
            "Key is not base64-encoded, using raw bytes (fingerprint: {})",
            Self::key_fingerprint(&raw)
        );
        raw
    }

    fn handle_keyring_error(e: KeyringError) -> Result<Vec<u8>> {
        match e {
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
        }
    }

    async fn get_local_key(&self) -> Result<Vec<u8>> {
        let credential_name = self.credential_name();
        debug!("Looking up keyring credential [name={credential_name}]");
        let entry = self.build_entry()?;

        match entry.get_secret() {
            Ok(raw) => {
                debug!("Credential found under session-scoped name [name={credential_name}]");
                Ok(Self::decode_key(raw))
            }
            Err(KeyringError::NoEntry) => {
                debug!("Credential not found under session-scoped name, trying legacy entry");
                self.get_local_key_fallback(entry).await
            }
            Err(e) => Self::handle_keyring_error(e),
        }
    }

    // Called when the session-scoped entry is missing.
    // Tries the legacy entry and migrates it, or creates a fresh key if neither exists.
    async fn get_local_key_fallback(&self, entry: Entry) -> Result<Vec<u8>> {
        match Self::build_legacy_entry()?.get_secret() {
            Ok(raw) => {
                debug!("Legacy credential found, migrating to session-scoped name");
                let key = Self::decode_key(raw);
                // Migrate: store under the new session-scoped name (base64-encoded).
                // Do not delete the old one in case another profile was using it
                let encoded = URL_SAFE_NO_PAD.encode(&key);
                match entry.set_secret(encoded.as_bytes()) {
                    Ok(()) => {
                        info!(
                            "Migrated keyring credential to session-scoped name (fingerprint: {})",
                            Self::key_fingerprint(&key)
                        );
                    }
                    Err(e) => {
                        info!("Could not migrate credential to session-scoped name: {e:?}");
                    }
                }
                Ok(key)
            }
            Err(KeyringError::NoEntry) => {
                debug!("No credential found in keyring (neither session-scoped nor legacy)");
                if self.session_exists() {
                    eprintln!(
                        "Error: Local encryption key not found but session exists. Forcing logout for security."
                    );
                    if let Err(logout_err) = crate::commands::logout::force_logout().await {
                        error!("Error during force logout: {logout_err:#}");
                    }
                    eprintln!("Run 'pass-cli login' to authenticate again.");
                    std::process::exit(1);
                } else {
                    info!("Credential not found in Keyring. Creating one");
                    let key = pass_domain::crypto::generate_encryption_key();
                    debug!(
                        "Generated new local key (fingerprint: {})",
                        Self::key_fingerprint(&key)
                    );
                    let encoded = URL_SAFE_NO_PAD.encode(&key);
                    entry
                        .set_secret(encoded.as_bytes())
                        .map_err(|e| anyhow::anyhow!("Error accessing keyring: {e}"))?;
                    info!("Stored credential into keyring");
                    Ok(key)
                }
            }
            Err(e) => Self::handle_keyring_error(e),
        }
    }
}

#[async_trait::async_trait]
impl LocalKeyProvider for KeyringKeyProvider {
    async fn get_key(&self) -> Result<LocalKey> {
        let key_guard = self.key.read().await;
        if let Some(key) = &*key_guard {
            debug!("Local key served from in-memory cache");
            Ok(LocalKey::new(xor_key_multibyte(key, &self.xor_key)))
        } else {
            drop(key_guard);
            let mut write_key_guard = self.key.write().await;
            if let Some(key) = &*write_key_guard {
                debug!("Local key served from in-memory cache (acquired after write lock)");
                return Ok(LocalKey::new(xor_key_multibyte(key, &self.xor_key)));
            }

            debug!("Local key not in cache, fetching from keyring");
            let key = self
                .get_local_key()
                .await
                .context("Could not get local key from keyring")?;

            debug!(
                "Local key loaded from keyring (fingerprint: {})",
                Self::key_fingerprint(&key)
            );
            let xored_key = xor_key_multibyte(&key, &self.xor_key);
            *write_key_guard = Some(xored_key);
            Ok(LocalKey::new(key))
        }
    }

    async fn remove_key(&self) -> Result<()> {
        let credential_name = self.credential_name();
        debug!("Removing keyring credential [name={credential_name}]");

        // Remove session-scoped entry.
        let entry = self.build_entry()?;
        if let Err(e) = entry.delete_credential()
            && !matches!(e, KeyringError::NoEntry)
        {
            return Err(anyhow::anyhow!("Error deleting credential: {e:?}"));
        }
        debug!("Session-scoped keyring credential removed [name={credential_name}]");

        // Also clean up any legacy entry that may still be present.
        debug!("Attempting to remove legacy keyring credential [name={KEYRING_CREDENTIAL_NAME}]");
        if let Err(e) = Self::build_legacy_entry()?.delete_credential()
            && !matches!(e, KeyringError::NoEntry)
        {
            info!("Could not delete legacy keyring credential: {e:?}");
        }

        Ok(())
    }
}
