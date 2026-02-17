use crate::storage::SessionStorage;
use anyhow::{Context, anyhow};
use muon::app::{AppName, AppVersion, SemVer};
use muon::client::Auth;
use muon::common::{Endpoint, Host, Server};
use muon::env::{Env, EnvId};
use muon::store::{Store, StoreError};
use muon::tls::{TlsCert, TlsPinSet, Verifier, VerifyRes};
use pass::PassSessionKeyType;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{AccountType, LocalKeyProvider};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AllowAllPinVerifier;

impl Verifier for AllowAllPinVerifier {
    fn verify(&self, _host: &Host, _head: &TlsCert, _tail: &[TlsCert]) -> muon::Result<VerifyRes> {
        Ok(VerifyRes::Accept)
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum CustomEnv {
    CustomUrl(String),
    Localhost,
}

impl Env for CustomEnv {
    fn servers(&self, _version: &AppVersion) -> Vec<Server> {
        match self {
            CustomEnv::CustomUrl(url) => {
                let without_start = url
                    .trim_start_matches("http://")
                    .trim_start_matches("https://");
                if without_start.contains("/") {
                    warn!("Path in custom url is not used. /api will be used")
                }
                let endpoint = Endpoint::from_str(url).expect("error parsing endpoint");
                vec![Server::new(endpoint, "/api")]
            }
            CustomEnv::Localhost => {
                let endpoint =
                    Endpoint::from_str("https://localhost").expect("error parsing endpoint");
                vec![Server::new(endpoint, "/api")]
            }
        }
    }

    fn pins(&self, _: &Server) -> Option<TlsPinSet> {
        None
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum SerializedEnv {
    Prod,
    Atlas(Option<String>),
    Custom(CustomEnv),
}

impl From<SerializedEnv> for EnvId {
    fn from(env: SerializedEnv) -> Self {
        match env {
            SerializedEnv::Prod => EnvId::Prod,
            SerializedEnv::Atlas(atlas) => EnvId::Atlas(atlas),
            SerializedEnv::Custom(env) => EnvId::new_custom(env),
        }
    }
}

impl From<EnvId> for SerializedEnv {
    fn from(env: EnvId) -> Self {
        match env {
            EnvId::Prod => SerializedEnv::Prod,
            EnvId::Atlas(atlas) => SerializedEnv::Atlas(atlas),
            EnvId::Custom(env) => {
                let servers = env.servers(&AppVersion::Named {
                    name: AppName::from_str("cli-pass").expect("Invalid AppName"),
                    version: SemVer::from_str(env!("CARGO_PKG_VERSION")).expect("Invalid SemVer"),
                });
                let endpoint = servers
                    .first()
                    .cloned()
                    .expect("should have one server")
                    .endpoint;
                let host_name = format!("{}", endpoint.host.name());
                if host_name == "localhost" {
                    SerializedEnv::Custom(CustomEnv::Localhost)
                } else {
                    SerializedEnv::Custom(CustomEnv::CustomUrl(format!("https://{endpoint}")))
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
    pub env: EnvId,
    pub auth: Arc<RwLock<Option<Auth>>>,
    pub storage: Arc<dyn SessionStorage>,
    pub key_provider: Arc<dyn LocalKeyProvider>,
    pub account_type: AccountType,
}

impl PassSessionStore {
    async fn inner_set_auth(&mut self, auth: Option<Auth>) -> Result<(), StoreError> {
        {
            let mut lock = self.auth.write().await;
            *lock = auth;
        }

        if let Err(e) = self.serialize().await {
            error!("Error serializing auth: {e:?}");
            Err(StoreError)
        } else {
            debug!("Session updated");
            Ok(())
        }
    }
}

#[async_trait::async_trait]
impl Store<PassSessionKeyType> for PassSessionStore {
    fn env(&self) -> EnvId {
        self.env.clone()
    }

    async fn get_auth(&self, _key: &PassSessionKeyType) -> Result<Auth, StoreError> {
        trace!("[STORE] PassSessionStore::get_auth()");
        let auth_data = self.auth.read().await;
        match auth_data.as_ref() {
            Some(auth) => Ok(auth.clone()),
            None => Ok(Auth::default()),
        }
    }

    async fn set_auth(&mut self, _key: &PassSessionKeyType, auth: Auth) -> Result<(), StoreError> {
        trace!("[STORE] PassSessionStore::set_auth()");
        self.inner_set_auth(Some(auth)).await
    }

    async fn remove_auth(&mut self, _key: &PassSessionKeyType) -> Result<Option<Auth>, StoreError> {
        trace!("[STORE] PassSessionStore::remove_auth()");

        let old_value = {
            let lock = self.auth.read().await;
            lock.clone()
        };

        self.inner_set_auth(None).await?;
        Ok(old_value)
    }

    async fn get_all_auth(&self) -> Result<HashMap<PassSessionKeyType, Auth>, StoreError> {
        trace!("[STORE] PassSessionStore::get_all_auth()");
        let lock = self.auth.read().await;

        let mut res = HashMap::new();
        if let Some(auth) = lock.as_ref() {
            res.insert((), auth.clone());
        }

        Ok(res)
    }

    async fn set_all_auth(
        &mut self,
        auth: HashMap<PassSessionKeyType, Auth>,
    ) -> Result<(), StoreError> {
        trace!("[STORE] PassSessionStore::set_all_auth()");
        if let Some(auth_value) = auth.get(&()) {
            self.inner_set_auth(Some(auth_value.clone())).await?;
        }
        Ok(())
    }

    async fn remove_all_auth(&mut self) -> Result<(), StoreError> {
        trace!("[STORE] PassSessionStore::remove_all_auth()");
        self.inner_set_auth(None).await?;
        Ok(())
    }
}

/// Wrapper around `Arc<RwLock<PassSessionStore>>` that implements Store
#[derive(Clone)]
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

#[async_trait::async_trait]
impl Store<PassSessionKeyType> for SharedPassSessionStore {
    fn env(&self) -> EnvId {
        // We need to block here since env() is not async
        // This is safe because env is read-only and cloned
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let store = self.inner.read().await;
                store.env.clone()
            })
        })
    }

    async fn get_auth(&self, key: &PassSessionKeyType) -> Result<Auth, StoreError> {
        let inner = self.inner.read().await;
        inner.get_auth(key).await
    }

    async fn set_auth(&mut self, key: &PassSessionKeyType, auth: Auth) -> Result<(), StoreError> {
        let mut inner = self.inner.write().await;
        inner.set_auth(key, auth).await
    }

    async fn remove_auth(&mut self, key: &PassSessionKeyType) -> Result<Option<Auth>, StoreError> {
        let mut inner = self.inner.write().await;
        inner.remove_auth(key).await
    }

    async fn get_all_auth(&self) -> Result<HashMap<PassSessionKeyType, Auth>, StoreError> {
        let inner = self.inner.read().await;
        inner.get_all_auth().await
    }

    async fn set_all_auth(
        &mut self,
        auth: HashMap<PassSessionKeyType, Auth>,
    ) -> Result<(), StoreError> {
        let mut inner = self.inner.write().await;
        inner.set_all_auth(auth).await
    }

    async fn remove_all_auth(&mut self) -> Result<(), StoreError> {
        let mut inner = self.inner.write().await;
        inner.remove_all_auth().await
    }
}

/// Error type for loading session store
pub enum GetStoreError {
    CannotDecrypt(anyhow::Error),
    Other(anyhow::Error),
}

impl PassSessionStore {
    pub fn new(
        env: EnvId,
        storage: Arc<dyn SessionStorage>,
        key_provider: Arc<dyn LocalKeyProvider>,
    ) -> Self {
        Self {
            auth: Arc::new(RwLock::new(None)),
            env,
            storage,
            key_provider,
            account_type: AccountType::User, // Default to User for new stores
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
            env: EnvId::from(deserialized.env),
            auth: Arc::new(RwLock::new(deserialized.auth)),
            storage,
            key_provider,
            account_type: deserialized.account_type,
        }))
    }

    pub async fn serialize(&self) -> anyhow::Result<()> {
        let serialized = {
            let auth = self.auth.read().await;
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

    pub async fn needs_extra_password(&self) -> bool {
        let auth = self.auth.read().await;
        if let Some(ref auth) = *auth {
            !auth.has_scope("pass")
        } else {
            false
        }
    }
}
