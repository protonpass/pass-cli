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

use crate::storage::SessionStorage;
use anyhow::{Context, anyhow};
use muon::app::{AppName, AppVersion, SemVer};
use muon::auth::Auth;
use muon::common::Server;
use muon::env::{Env, Environment};
use muon::store::Store;
use muon::tls::pins::TlsPinSet;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{AccountType, LocalKeyProvider};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

pub type PassSessionKeyType = ();

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum CustomEnv {
    CustomUrl(String),
    Localhost,
}

impl Env for CustomEnv {
    fn servers(&self, _version: &AppVersion) -> Vec<Server> {
        match self {
            CustomEnv::CustomUrl(url) => {
                let server_url = if url.ends_with("/api") || url.contains("/api/") {
                    url.to_string()
                } else {
                    // Strip any trailing path and use /api
                    let base = url
                        .trim_start_matches("http://")
                        .trim_start_matches("https://");
                    if base.contains("/") {
                        warn!("Path in custom url is not used. /api will be used");
                    }
                    let host_port = base.split('/').next().unwrap_or(base);
                    let scheme = if url.starts_with("http://") {
                        "http"
                    } else {
                        "https"
                    };
                    format!("{scheme}://{host_port}/api")
                };
                let server: Server = server_url.parse().expect("error parsing server URL");
                vec![server]
            }
            CustomEnv::Localhost => {
                let server: Server = "https://localhost/api"
                    .parse()
                    .expect("error parsing localhost");
                vec![server]
            }
        }
    }

    fn ar_pins(&self) -> Option<&TlsPinSet> {
        None
    }

    fn api_pins(&self) -> Option<&TlsPinSet> {
        None
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum SerializedEnv {
    Prod,
    Atlas(Option<String>),
    Custom(CustomEnv),
}

impl From<SerializedEnv> for Environment {
    fn from(env: SerializedEnv) -> Self {
        match env {
            SerializedEnv::Prod => Environment::new_prod(),
            SerializedEnv::Atlas(None) => Environment::new_atlas(),
            SerializedEnv::Atlas(Some(name)) => Environment::new_atlas_name(name),
            SerializedEnv::Custom(env) => Environment::new_custom(env),
        }
    }
}

impl From<Environment> for SerializedEnv {
    fn from(env: Environment) -> Self {
        match env {
            Environment::Prod(_) => SerializedEnv::Prod,
            Environment::Atlas(_) => SerializedEnv::Atlas(None),
            Environment::Scientist(s) => {
                // AtlasScientist wraps a Scientist(String). Access via pattern.
                // We need the inner name - use the servers() call to extract host info
                let servers = s.servers(&AppVersion::Named {
                    name: AppName::from_str("cli-pass").expect("Invalid AppName"),
                    version: SemVer::from_str(env!("CARGO_PKG_VERSION")).expect("Invalid SemVer"),
                });
                debug!("SerializedEnv Servers: {servers:?}");
                let host_str = servers
                    .first()
                    .map(|srv| format!("{}", srv.host().name()))
                    .unwrap_or_default();
                debug!("SerializedEnv host_str: {host_str:?}");
                // host format is "{product}-api.{name}.proton.black" or "{name}.proton.black"
                // extract the scientist name
                let name = crate::utils::extract_scientist_name(&host_str);
                debug!("SerializedEnv name: {host_str:?}");
                SerializedEnv::Atlas(Some(name))
            }
            Environment::Custom(env) => {
                let servers = env.servers(&AppVersion::Named {
                    name: AppName::from_str("cli-pass").expect("Invalid AppName"),
                    version: SemVer::from_str(env!("CARGO_PKG_VERSION")).expect("Invalid SemVer"),
                });
                let server = servers.first().cloned().expect("should have one server");
                let host_name = format!("{}", server.host().name());
                if host_name == "localhost" {
                    SerializedEnv::Custom(CustomEnv::Localhost)
                } else {
                    let scheme = server.scheme();
                    let port = server.port();
                    SerializedEnv::Custom(CustomEnv::CustomUrl(format!(
                        "{scheme}://{host_name}:{port}"
                    )))
                }
            }
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct SerializedStore {
    pub auth: Option<Auth>,
    pub env: SerializedEnv,
    #[serde(default = "default_account_type")]
    pub account_type: AccountType,
}

fn default_account_type() -> AccountType {
    AccountType::User
}

#[derive(Clone)]
pub struct PassSessionStore {
    pub env: Environment,
    pub auth: Arc<Mutex<Option<Auth>>>,
    pub storage: Arc<dyn SessionStorage>,
    pub key_provider: Arc<dyn LocalKeyProvider>,
    pub account_type: AccountType,
    persist_generation: Arc<AtomicU64>,
    persist_lock: Arc<tokio::sync::Mutex<()>>,
}

impl std::fmt::Debug for PassSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassSessionStore")
            .field("env", &self.env)
            .field("account_type", &self.account_type)
            .finish()
    }
}

impl PassSessionStore {
    fn schedule_persist(&self) {
        let generation = self.persist_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let store_clone = self.clone();
        tokio::spawn(async move {
            if let Err(e) = store_clone.persist_if_latest(generation).await {
                error!("Error serializing auth: {e:?}");
            } else {
                debug!("Session updated");
            }
        });
    }

    async fn persist_if_latest(&self, generation: u64) -> anyhow::Result<()> {
        let _guard = self.persist_lock.lock().await;

        if self.persist_generation.load(Ordering::SeqCst) != generation {
            debug!("Skipping stale session persistence request");
            return Ok(());
        }

        self.serialize().await
    }

    pub async fn persist_now(&self) -> anyhow::Result<()> {
        self.persist_generation.fetch_add(1, Ordering::SeqCst);
        let _guard = self.persist_lock.lock().await;
        self.serialize().await
    }

    fn inner_set_auth(&mut self, auth: Option<Auth>) {
        {
            let mut lock = self.auth.lock().expect("auth mutex poisoned");
            *lock = auth;
        }

        self.schedule_persist();
    }
}

impl Store for PassSessionStore {
    type Key = PassSessionKeyType;

    fn set_auth(&mut self, _key: PassSessionKeyType, auth: Auth) {
        trace!("[STORE] PassSessionStore::set_auth()");
        self.inner_set_auth(Some(auth));
    }

    fn remove_auth(&mut self, _key: &PassSessionKeyType) {
        trace!("[STORE] PassSessionStore::remove_auth()");
        self.inner_set_auth(None);
    }

    fn get_all_auth(&self) -> HashMap<PassSessionKeyType, Auth> {
        trace!("[STORE] PassSessionStore::get_all_auth()");
        let lock = self.auth.lock().expect("auth mutex poisoned");
        let mut res = HashMap::new();
        if let Some(auth) = lock.as_ref() {
            res.insert((), auth.clone());
        }
        res
    }
}

/// Wrapper around `Arc<RwLock<PassSessionStore>>` that implements Store
#[derive(Clone, Debug)]
pub struct SharedPassSessionStore {
    pub inner: Arc<RwLock<PassSessionStore>>,
}

impl SharedPassSessionStore {
    pub fn new(store: PassSessionStore) -> Self {
        Self {
            inner: Arc::new(RwLock::new(store)),
        }
    }
}

impl Store for SharedPassSessionStore {
    type Key = PassSessionKeyType;

    fn set_auth(&mut self, key: PassSessionKeyType, auth: Auth) {
        let mut inner = self.inner.write().expect("store rwlock poisoned");
        inner.set_auth(key, auth);
    }

    fn remove_auth(&mut self, key: &PassSessionKeyType) {
        let mut inner = self.inner.write().expect("store rwlock poisoned");
        inner.remove_auth(key);
    }

    fn get_all_auth(&self) -> HashMap<PassSessionKeyType, Auth> {
        let inner = self.inner.read().expect("store rwlock poisoned");
        inner.get_all_auth()
    }
}

/// Error type for loading session store
pub enum GetStoreError {
    CannotDecrypt(anyhow::Error),
    Other(anyhow::Error),
}

impl PassSessionStore {
    pub fn new(
        env: Environment,
        storage: Arc<dyn SessionStorage>,
        key_provider: Arc<dyn LocalKeyProvider>,
    ) -> Self {
        Self {
            auth: Arc::new(Mutex::new(None)),
            env,
            storage,
            key_provider,
            account_type: AccountType::User, // Default to User for new stores
            persist_generation: Arc::new(AtomicU64::new(0)),
            persist_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn set_account_type(&mut self, account_type: AccountType) {
        self.account_type = account_type;
    }

    pub fn account_type(&self) -> AccountType {
        self.account_type
    }

    pub async fn get_from_local(
        storage: Arc<dyn SessionStorage>,
        key_provider: Arc<dyn LocalKeyProvider>,
    ) -> Result<Option<PassSessionStore>, GetStoreError> {
        // Try to load encrypted data from storage
        let contents = match storage.load().await {
            Ok(Some(data)) => data,
            Ok(None) => return Ok(None), // No session stored
            Err(e) => {
                return Err(GetStoreError::Other(anyhow!(
                    "Error loading from storage: {e}"
                )));
            }
        };

        // Decrypt the session data
        let local_key = match key_provider.get_key().await {
            Ok(k) => k,
            Err(e) => {
                return Err(GetStoreError::Other(anyhow!(
                    "Error getting local key: {e}"
                )));
            }
        };

        let decrypted = match pass_domain::crypto::decrypt(
            &contents,
            local_key.as_ref(),
            EncryptionTag::Unknown,
        ) {
            Ok(decrypted) => decrypted,
            Err(e) => {
                return Err(GetStoreError::CannotDecrypt(anyhow!(
                    "Error decrypting session: {e}"
                )));
            }
        };

        let deserialized: SerializedStore = match serde_json::from_slice(&decrypted) {
            Ok(s) => s,
            Err(e) => {
                return Err(GetStoreError::Other(anyhow!(
                    "Error deserializing json: {e}"
                )));
            }
        };

        Ok(Some(PassSessionStore {
            env: Environment::from(deserialized.env),
            auth: Arc::new(Mutex::new(deserialized.auth)),
            storage,
            key_provider,
            account_type: deserialized.account_type,
            persist_generation: Arc::new(AtomicU64::new(0)),
            persist_lock: Arc::new(tokio::sync::Mutex::new(())),
        }))
    }

    pub async fn serialize(&self) -> anyhow::Result<()> {
        let serialized = {
            let auth = self.auth.lock().expect("auth mutex poisoned");
            SerializedStore {
                env: SerializedEnv::from(self.env.clone()),
                auth: auth.clone(),
                account_type: self.account_type,
            }
        };

        debug!("[STORE] Storing session");

        let as_str = serde_json::to_string(&serialized).context("Error serializing json")?;

        let local_key = self
            .key_provider
            .get_key()
            .await
            .context("Error getting local key")?;

        let encrypted = match pass_domain::crypto::encrypt(
            as_str.as_bytes(),
            local_key.as_ref(),
            EncryptionTag::Unknown,
        ) {
            Ok(encrypted) => encrypted,
            Err(e) => {
                return Err(anyhow!("Error encrypting session: {}", e));
            }
        };

        // Delegate to the storage implementation
        self.storage
            .save(&encrypted)
            .await
            .context("Error saving session to storage")?;

        debug!("[STORE] Stored session");

        Ok(())
    }

    pub fn needs_extra_password(&self) -> bool {
        let auth = self.auth.lock().expect("auth mutex poisoned");
        if let Some(ref auth) = *auth {
            !auth.has_scope("pass")
        } else {
            false
        }
    }
}
