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

use crate::auth::config::HOST_LOOKUP_APP_NAME;
use crate::auth::storage::SessionStorage;
use crate::domain::crypto::EncryptionTag;
use crate::domain::{AccountType, LocalKeyProvider};
use anyhow::{Context, anyhow};
use futures::task::{FutureObj, Spawn};
use muon::app::{AppName, AppVersion, SemVer};
use muon::auth::Auth;
use muon::common::Server;
use muon::cookie_store::StoredCookie;
use muon::env::{Env, Environment};
use muon::store::Store;
use muon::tls::pins::TlsPinSet;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::client::PassSessionKeyType;

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
                    name: AppName::from_str(HOST_LOOKUP_APP_NAME).expect("Invalid AppName"),
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
                let name = crate::auth::utils::extract_scientist_name(&host_str);
                debug!("SerializedEnv name: {host_str:?}");
                SerializedEnv::Atlas(Some(name))
            }
            Environment::Custom(env) => {
                let servers = env.servers(&AppVersion::Named {
                    name: AppName::from_str(HOST_LOOKUP_APP_NAME).expect("Invalid AppName"),
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
    #[serde(default = "default_lock_after_seconds")]
    pub session_lock_after_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cookies: Vec<StoredCookie>,
}

fn default_account_type() -> AccountType {
    AccountType::User
}

const fn default_lock_after_seconds() -> Option<u32> {
    None
}

#[derive(Clone)]
pub struct PassSessionStore {
    pub env: Environment,
    pub auth: Arc<Mutex<Option<Auth>>>,
    pub storage: Arc<dyn SessionStorage>,
    pub key_provider: Arc<dyn LocalKeyProvider>,
    pub account_type: AccountType,
    pub session_lock_after_seconds: Option<u32>,
    pub cookies: Arc<RwLock<Vec<StoredCookie>>>,
    executor: Arc<dyn futures::task::Spawn + Send + Sync>,
    persist_generation: Arc<AtomicU64>,
    persisted_generation: Arc<AtomicU64>,
    persist_lock: Arc<async_lock::Mutex<()>>,
}

impl std::fmt::Debug for PassSessionStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PassSessionStore")
            .field("env", &self.env)
            .field("account_type", &self.account_type)
            .field(
                "session_lock_after_seconds",
                &self.session_lock_after_seconds,
            )
            .field("cookies", &self.cookies.read().len())
            .finish()
    }
}

impl PassSessionStore {
    pub(crate) fn schedule_persist(&self) {
        self.persist_generation.fetch_add(1, Ordering::SeqCst);

        let store_clone = self.clone();
        let persist = FutureObj::new(Box::pin(async move {
            if let Err(e) = store_clone.flush().await {
                error!("Error serializing auth: {e:?}");
            }
        }));
        if let Err(e) = self.executor.spawn_obj(persist) {
            error!("Failed to schedule session persist: {e:?}");
        }
    }

    pub async fn flush(&self) -> anyhow::Result<()> {
        let _guard = self.persist_lock.lock().await;

        let generation = self.persist_generation.load(Ordering::SeqCst);
        if self.persisted_generation.load(Ordering::SeqCst) == generation {
            return Ok(());
        }

        self.serialize().await?;
        self.persisted_generation
            .store(generation, Ordering::SeqCst);
        Ok(())
    }

    pub async fn persist_now(&self) -> anyhow::Result<()> {
        self.persist_generation.fetch_add(1, Ordering::SeqCst);
        self.flush().await
    }

    fn inner_set_auth(&mut self, auth: Option<Auth>) {
        {
            let mut lock = self.auth.lock();
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
        let lock = self.auth.lock();
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
        let mut inner = self.inner.write();
        inner.set_auth(key, auth);
    }

    fn remove_auth(&mut self, key: &PassSessionKeyType) {
        let mut inner = self.inner.write();
        inner.remove_auth(key);
    }

    fn get_all_auth(&self) -> HashMap<PassSessionKeyType, Auth> {
        let inner = self.inner.read();
        inner.get_all_auth()
    }
}

pub type PassMuonSessionStore = muon::store::PersistedSessionStore<SharedPassSessionStore>;

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
        executor: Arc<dyn Spawn + Send + Sync>,
    ) -> Self {
        Self {
            auth: Arc::new(Mutex::new(None)),
            env,
            storage,
            key_provider,
            account_type: AccountType::User, // Default to User for new stores
            session_lock_after_seconds: None,
            cookies: Arc::new(RwLock::new(Vec::new())),
            executor,
            persist_generation: Arc::new(AtomicU64::new(0)),
            persisted_generation: Arc::new(AtomicU64::new(0)),
            persist_lock: Arc::new(async_lock::Mutex::new(())),
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
        executor: Arc<dyn Spawn + Send + Sync>,
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

        let decrypted = match crate::domain::crypto::decrypt(
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
            session_lock_after_seconds: deserialized.session_lock_after_seconds,
            cookies: Arc::new(RwLock::new(deserialized.cookies)),
            executor,
            persist_generation: Arc::new(AtomicU64::new(0)),
            persisted_generation: Arc::new(AtomicU64::new(0)),
            persist_lock: Arc::new(async_lock::Mutex::new(())),
        }))
    }

    pub async fn serialize(&self) -> anyhow::Result<()> {
        let serialized = {
            let auth = self.auth.lock();
            SerializedStore {
                env: SerializedEnv::from(self.env.clone()),
                auth: auth.clone(),
                account_type: self.account_type,
                session_lock_after_seconds: self.session_lock_after_seconds,
                cookies: self.cookies.read().clone(),
            }
        };

        debug!("[STORE] Storing session");

        let as_str = serde_json::to_string(&serialized).context("Error serializing json")?;

        let local_key = self
            .key_provider
            .get_key()
            .await
            .context("Error getting local key")?;

        let encrypted = match crate::domain::crypto::encrypt(
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
        let auth = self.auth.lock();
        if let Some(ref auth) = *auth {
            !auth.has_scope("pass")
        } else {
            false
        }
    }

    pub fn get_session_lock_after_seconds(&self) -> Option<u32> {
        self.session_lock_after_seconds
    }

    pub fn set_session_lock_after_seconds(&mut self, lock_time: Option<u32>) {
        self.session_lock_after_seconds = lock_time;
        self.schedule_persist();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use muon::cookie_store::{CookieExpiration, UnixTimestamp};

    fn cookie(name: &str) -> StoredCookie {
        StoredCookie {
            name: name.to_owned(),
            value: "value".to_owned(),
            secure: false,
            http_only: false,
            origin_url: "https://proton.me/".parse().unwrap(),
            domain_attribute: None,
            path_attribute: Some("/".to_owned()),
            expiration: CookieExpiration::AtUtc(UnixTimestamp::from_secs(1_900_000_000)),
        }
    }

    #[test]
    fn session_without_cookies_still_deserializes() {
        let json =
            r#"{"auth":null,"env":"Prod","account_type":"User","session_lock_after_seconds":null}"#;

        let store: SerializedStore = serde_json::from_str(json).expect("old session must load");

        assert!(store.cookies.is_empty());
        assert_eq!(store.account_type, AccountType::User);
    }

    #[test]
    fn cookies_round_trip() {
        let store = SerializedStore {
            auth: None,
            env: SerializedEnv::Prod,
            account_type: AccountType::User,
            session_lock_after_seconds: None,
            cookies: vec![cookie("Session-Id")],
        };

        let json = serde_json::to_string(&store).unwrap();
        let read: SerializedStore = serde_json::from_str(&json).unwrap();

        assert_eq!(read.cookies, store.cookies);
    }

    #[derive(Default)]
    struct MemoryStorage {
        contents: Mutex<Option<Vec<u8>>>,
    }

    #[async_trait::async_trait]
    impl SessionStorage for MemoryStorage {
        async fn load(&self) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(self.contents.lock().clone())
        }
        async fn save(&self, data: &[u8]) -> anyhow::Result<()> {
            *self.contents.lock() = Some(data.to_vec());
            Ok(())
        }
        async fn exists(&self) -> bool {
            self.contents.lock().is_some()
        }
        async fn delete(&self) -> anyhow::Result<()> {
            *self.contents.lock() = None;
            Ok(())
        }
    }

    struct FixedKeyProvider(Vec<u8>);

    #[async_trait::async_trait]
    impl LocalKeyProvider for FixedKeyProvider {
        async fn get_key(&self) -> anyhow::Result<crate::domain::LocalKey> {
            Ok(crate::domain::LocalKey::new(self.0.clone()))
        }
        async fn remove_key(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    fn key_provider() -> Arc<dyn LocalKeyProvider> {
        Arc::new(FixedKeyProvider(
            crate::domain::crypto::generate_encryption_key(),
        ))
    }

    struct TestExecutor;

    impl Spawn for TestExecutor {
        fn spawn_obj(
            &self,
            future: FutureObj<'static, ()>,
        ) -> Result<(), futures::task::SpawnError> {
            tokio::spawn(future);
            Ok(())
        }
    }

    fn executor() -> Arc<dyn Spawn + Send + Sync> {
        Arc::new(TestExecutor)
    }

    #[tokio::test]
    async fn cookies_survive_a_save_load_round_trip() {
        let storage = Arc::new(MemoryStorage::default());
        let key_provider = key_provider();

        let store = PassSessionStore::new(
            Environment::new_prod(),
            storage.clone(),
            key_provider.clone(),
            executor(),
        );
        store.cookies.write().push(cookie("Session-Id"));
        store.persist_now().await.expect("session must be written");

        let loaded = match PassSessionStore::get_from_local(storage, key_provider, executor()).await
        {
            Ok(Some(store)) => store,
            Ok(None) => panic!("a session was written, so one must be loaded"),
            Err(GetStoreError::CannotDecrypt(e)) => panic!("session must decrypt: {e:#}"),
            Err(GetStoreError::Other(e)) => panic!("session must load: {e:#}"),
        };

        assert_eq!(loaded.cookies.read().as_slice(), [cookie("Session-Id")]);
    }

    struct IdleExecutor;

    impl Spawn for IdleExecutor {
        fn spawn_obj(
            &self,
            _future: FutureObj<'static, ()>,
        ) -> Result<(), futures::task::SpawnError> {
            Ok(())
        }
    }

    fn idle_store(storage: Arc<MemoryStorage>) -> PassSessionStore {
        PassSessionStore::new(
            Environment::new_prod(),
            storage,
            key_provider(),
            Arc::new(IdleExecutor),
        )
    }

    #[tokio::test]
    async fn flush_without_changes_does_not_write() {
        let storage = Arc::new(MemoryStorage::default());
        let store = idle_store(storage.clone());

        store.flush().await.unwrap();

        assert!(storage.contents.lock().is_none());
    }

    #[tokio::test]
    async fn flush_writes_pending_changes_once() {
        let storage = Arc::new(MemoryStorage::default());
        let store = idle_store(storage.clone());

        store.schedule_persist();
        store.schedule_persist();
        store.flush().await.unwrap();
        assert!(storage.contents.lock().take().is_some());

        store.flush().await.unwrap();
        assert!(storage.contents.lock().is_none());
    }

    #[tokio::test]
    async fn muon_session_store_writes_through_to_pass_store() {
        use muon::store::{PersistedSessionStore, SessionRead, SessionStore, SessionWrite};

        let storage = Arc::new(MemoryStorage::default());
        let shared = SharedPassSessionStore::new(idle_store(storage.clone()));
        let auth_mirror = shared.inner.read().auth.clone();
        let muon_store = PersistedSessionStore::new(shared.clone());

        {
            let mut guard = muon_store.write(&()).await.unwrap();
            guard
                .replace(Auth::external("user", "uid-1"))
                .await
                .unwrap();
        }
        assert_eq!(
            auth_mirror
                .lock()
                .as_ref()
                .and_then(|a| a.uid().map(str::to_owned)),
            Some("uid-1".to_owned())
        );
        assert_eq!(muon_store.keys().await.len(), 1);
        {
            let guard = muon_store.read(&()).await.unwrap();
            assert_eq!(guard.auth().uid(), Some("uid-1"));
        }

        let snapshot = shared.inner.read().clone();
        snapshot.flush().await.unwrap();
        assert!(storage.contents.lock().is_some());

        {
            let mut guard = muon_store.write(&()).await.unwrap();
            assert!(guard.remove().await.is_some());
        }
        assert!(auth_mirror.lock().is_none());
        assert!(muon_store.keys().await.is_empty());
    }

    #[tokio::test]
    async fn muon_session_store_loads_existing_session() {
        use muon::store::{PersistedSessionStore, SessionRead, SessionStore};

        let shared = SharedPassSessionStore::new(idle_store(Arc::new(MemoryStorage::default())));
        *shared.inner.read().auth.lock() = Some(Auth::external("user", "uid-2"));

        let muon_store = PersistedSessionStore::new(shared);

        let guard = muon_store.read(&()).await.unwrap();
        assert_eq!(guard.auth().uid(), Some("uid-2"));
    }

    #[test]
    fn empty_cookies_are_not_serialized() {
        let store = SerializedStore {
            auth: None,
            env: SerializedEnv::Prod,
            account_type: AccountType::User,
            session_lock_after_seconds: None,
            cookies: Vec::new(),
        };

        let json = serde_json::to_string(&store).unwrap();

        assert!(
            !json.contains("cookies"),
            "an empty jar must not add noise to the session file: {json}"
        );
    }
}
