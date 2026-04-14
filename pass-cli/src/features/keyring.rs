/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::constants::SESSION_FILE_NAME;
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use keyring_core::{Entry, Error as KeyringError};
use pass_db::DATABASE_NAME;
use pass_domain::utils::xor_key_multibyte;
use pass_domain::{LocalKey, LocalKeyProvider};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::sync::RwLock;

const KEYRING_SERVICE_NAME: &str = "ProtonPassCLI";
const KEYRING_CREDENTIAL_NAME: &str = "cli-local-key";
const XOR_KEY_LENGTH: usize = 32;

#[cfg(target_os = "linux")]
const LINUX_KEYRING_BACKEND_ENV_VAR: &str = "PROTON_PASS_LINUX_KEYRING";

#[cfg(target_os = "linux")]
enum LinuxKeyringBackend {
    Dbus,
    Kernel,
}

#[cfg(target_os = "linux")]
impl LinuxKeyringBackend {
    fn from_env() -> Self {
        match std::env::var(LINUX_KEYRING_BACKEND_ENV_VAR) {
            Ok(val) => match val.to_ascii_lowercase().as_str() {
                "dbus" => Self::Dbus,
                "kernel" => Self::Kernel,
                other => {
                    warn!(
                        "Linux keyring: unknown value '{other}' for {LINUX_KEYRING_BACKEND_ENV_VAR}, \
                    falling back to kernel keyutils. Valid values are: dbus, kernel"
                    );
                    Self::Kernel
                }
            },
            Err(_) => Self::Kernel,
        }
    }
}

#[cfg(target_os = "linux")]
fn init_linux_store() -> Result<()> {
    match LinuxKeyringBackend::from_env() {
        LinuxKeyringBackend::Dbus => {
            info!(
                "Linux keyring: D-Bus backend requested via {LINUX_KEYRING_BACKEND_ENV_VAR}=dbus"
            );
            let store = zbus_secret_service_keyring_store::Store::new().map_err(|e| {
                anyhow::anyhow!(
                    "Linux keyring: D-Bus secret service is unavailable or locked. \
                    Make sure your desktop session is unlocked and the Secret Service \
                    (e.g. GNOME Keyring) is running: {e}"
                )
            })?;
            keyring_core::set_default_store(store);
            info!("Linux keyring: using zbus secret service (persistent)");
        }
        LinuxKeyringBackend::Kernel => {
            let store = linux_keyutils_keyring_store::Store::new()
                .map_err(|e| anyhow::anyhow!("Failed to initialize kernel keyutils store: {e}"))?;
            keyring_core::set_default_store(store);
            info!("Linux keyring: using kernel keyutils (cleared on reboot)");
        }
    }

    Ok(())
}

fn init_keyring_store() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let store = apple_native_keyring_store::keychain::Store::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize macOS keychain store: {e}"))?;
        keyring_core::set_default_store(store);
    }

    #[cfg(target_os = "windows")]
    {
        let store = windows_native_keyring_store::Store::new()
            .map_err(|e| anyhow::anyhow!("Failed to initialize Windows keyring store: {e}"))?;
        keyring_core::set_default_store(store);
    }

    #[cfg(target_os = "linux")]
    init_linux_store()?;

    Ok(())
}

pub struct KeyringKeyProvider {
    key: RwLock<Option<Vec<u8>>>,
    xor_key: Vec<u8>,
    base_dir: PathBuf,
}

impl KeyringKeyProvider {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        init_keyring_store()?;
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

    // Returns true if any local state that was encrypted with the stored key exists.
    // This is broader than `session_exists` because the database is created before
    // session.json is written. If the process is killed between those two steps, the
    // database exists without a session file. On the next start, if the keyring key is
    // also gone (e.g. Linux reboot with kernel keyring), we must not generate a new key
    // and try to open the existing database with it, as that would cause an HMAC failure.
    fn local_data_exists(&self) -> bool {
        self.session_exists() || self.base_dir.join(DATABASE_NAME).is_file()
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
                if self.local_data_exists() {
                    eprintln!(
                        "Error: Local encryption key not found but local data exists. Forcing logout for security."
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
