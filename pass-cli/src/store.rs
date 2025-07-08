use anyhow::{Context, anyhow};
use muon::app::AppVersion;
use muon::client::Auth;
use muon::common::{Endpoint, Host, Server};
use muon::env::{Env, EnvId};
use muon::store::{Store, StoreError};
use muon::tls::{TlsCert, TlsPinSet, Verifier, VerifyRes};
use pass_domain::crypto::EncryptionTag;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const FILE_NAME: &str = "session.json";

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
                let servers = env.servers(&AppVersion::Other);
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
    pub auth: Auth,
    pub env: SerializedEnv,
}

#[derive(Clone)]
pub struct AuthenticatorStore {
    pub env: EnvId,
    pub auth: Arc<RwLock<Auth>>,
    pub base_path: PathBuf,
}

#[async_trait::async_trait]
impl Store for AuthenticatorStore {
    fn env(&self) -> EnvId {
        self.env.clone()
    }

    async fn get_auth(&self) -> Auth {
        trace!("[STORE] AuthenticatorStore::get_auth()");
        let lock = self.auth.read().await;
        lock.clone()
    }

    async fn set_auth(&mut self, auth: Auth) -> anyhow::Result<Auth, StoreError> {
        trace!("[STORE] AuthenticatorStore::set_auth()");
        {
            let mut lock = self.auth.write().await;
            *lock = auth.clone();
        }

        if let Err(e) = self.serialize().await {
            error!("Error serializing auth: {:?}", e);
            Err(StoreError)
        } else {
            debug!("Session updated");
            Ok(auth)
        }
    }
}

impl AuthenticatorStore {
    pub fn new_with_path(env: EnvId, base_path: PathBuf) -> Self {
        Self {
            auth: Arc::new(RwLock::new(Auth::default())),
            env,
            base_path,
        }
    }

    pub async fn get_from_local(base_path: PathBuf) -> anyhow::Result<Option<AuthenticatorStore>> {
        let file_path = base_path.join(FILE_NAME);
        if !file_path.exists() || !file_path.is_file() {
            return Ok(None);
        }

        let contents = std::fs::read(file_path).context("Error reading file")?;
        let local_key = crate::storage::get_local_key(&base_path)
            .await
            .context("Error getting local key")?;

        let decrypted =
            match pass_domain::crypto::decrypt(&contents, &local_key, EncryptionTag::Unknown) {
                Ok(decrypted) => decrypted,
                Err(e) => {
                    return Err(anyhow!("Error decrypting session: {}", e));
                }
            };

        let deserialized: SerializedStore =
            serde_json::from_slice(&decrypted).context("Error deserializing json")?;

        Ok(Some(AuthenticatorStore {
            env: EnvId::from(deserialized.env),
            auth: Arc::new(RwLock::new(deserialized.auth)),
            base_path,
        }))
    }

    pub async fn serialize(&self) -> anyhow::Result<()> {
        let serialized = {
            let auth = self.auth.read().await;
            SerializedStore {
                env: SerializedEnv::from(self.env.clone()),
                auth: auth.clone(),
            }
        };

        let file_path = self.base_path.join(FILE_NAME);
        debug!("[STORE] Storing session session to {}", file_path.display());

        let as_str = serde_json::to_string(&serialized).context("Error serializing json")?;
        let local_key = crate::storage::get_local_key(&self.base_path)
            .await
            .context("Error getting local key")?;

        let encrypted = match pass_domain::crypto::encrypt(
            as_str.as_bytes(),
            &local_key,
            EncryptionTag::Unknown,
        ) {
            Ok(encrypted) => encrypted,
            Err(e) => {
                return Err(anyhow!("Error encrypting session: {}", e));
            }
        };

        tokio::fs::write(file_path, encrypted)
            .await
            .context("Error writing file")?;

        debug!("[STORE] Stored session");

        Ok(())
    }
}
