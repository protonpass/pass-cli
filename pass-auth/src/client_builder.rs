use crate::config::ClientConfig;
use crate::storage::SessionStorage;
use crate::store::{
    AllowAllPinVerifier, CustomEnv, GetStoreError, PassSessionStore, SerializedEnv,
    SharedPassSessionStore,
};
use anyhow::{Context, anyhow};
use muon::app::{App, AppVersion};
use muon::common::{BoxFut, EnvProxy, Sender, SenderLayer};
use muon::env::{Env, EnvId};
use muon::{ProtonRequest, ProtonResponse};
use pass::Client;
use pass_domain::LocalKeyProvider;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const ENVIRONMENT_ENV_VAR: &str = "PROTON_PASS_ENVIRONMENT";
const XDEBUG_SESSION_HEADER: &str = "XDEBUG_SESSION";
const APP_NAME: &str = "cli-pass";

struct XdebugSessionLayer {
    session: String,
}

impl XdebugSessionLayer {
    pub fn new(session: String) -> Self {
        Self { session }
    }
}

impl SenderLayer<ProtonRequest, ProtonResponse> for XdebugSessionLayer {
    fn on_send<'a>(
        &'a self,
        inner: &'a dyn Sender<ProtonRequest, ProtonResponse>,
        req: ProtonRequest,
    ) -> BoxFut<'a, muon::Result<ProtonResponse>> {
        Box::pin(async move {
            let with_header = req.header((XDEBUG_SESSION_HEADER, self.session.clone()));
            inner.send(with_header).await
        })
    }
}

fn get_env(config: &ClientConfig) -> SerializedEnv {
    let env_string = config
        .environment
        .clone()
        .or_else(|| std::env::var(ENVIRONMENT_ENV_VAR).ok())
        .unwrap_or_else(|| "prod".to_string());

    let env_str = env_string.as_str();

    match env_str {
        "prod" => SerializedEnv::Prod,
        "atlas" => SerializedEnv::Atlas(None),
        "localhost" => SerializedEnv::Custom(CustomEnv::Localhost),
        s if s.starts_with("http") => SerializedEnv::Custom(CustomEnv::CustomUrl(s.to_string())),
        s => SerializedEnv::Atlas(Some(s.to_string())),
    }
}

fn store_using_current_env(store_env: &EnvId, current_env: &EnvId) -> bool {
    match current_env {
        EnvId::Prod => matches!(store_env, EnvId::Prod),
        EnvId::Custom(_) => matches!(store_env, EnvId::Custom(_)),
        EnvId::Atlas(current_atlas) => match store_env {
            EnvId::Atlas(store_atlas) => store_atlas == current_atlas,
            _ => false,
        },
    }
}

pub async fn create_client(
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    config: &ClientConfig,
) -> anyhow::Result<(Client, Arc<RwLock<PassSessionStore>>)> {
    // Check key_provider can be used
    key_provider
        .get_key()
        .await
        .context("Error accessing key provider")?;

    let app_header = config
        .app_header
        .as_ref()
        .cloned()
        .unwrap_or_else(|| format!("{}@{}", APP_NAME, env!("CARGO_PKG_VERSION")));
    let app = App::new(app_header).context("failed to create app")?;

    // Load or create session store
    let store = match PassSessionStore::get_from_local(storage.clone(), key_provider.clone()).await
    {
        Ok(store) => store,
        Err(e) => {
            return match e {
                GetStoreError::CannotDecrypt(e) => Err(anyhow!(
                    "Error decrypting local session({e:#}). Make sure you have not changed your key provider / removed your local key, or try to logout and log in again"
                )),
                GetStoreError::Other(e) => Err(anyhow!("Error loading local session: {e:#}")),
            };
        }
    };

    let current_env = EnvId::from(get_env(config));

    let store = store.unwrap_or_else(|| {
        debug!("Using env {current_env:?}");
        PassSessionStore::new(current_env.clone(), storage.clone(), key_provider)
    });

    // Check for environment switch
    if !store_using_current_env(&store.env, &current_env) {
        return Err(anyhow!(
            "ENVIRONMENT has switched! Please log out and log back in again with the new environment"
        ));
    }

    // Determine if we need AllowAllPinVerifier (for localhost)
    let mut use_allow_all = false;
    if let EnvId::Custom(ref env) = store.env
        && let Some(server) = env.servers(&AppVersion::Other).first()
    {
        let host_name = format!("{}", server.endpoint.host.name());
        if host_name == "localhost" {
            use_allow_all = true;
        }
    }

    // Build the client
    let shared_store = SharedPassSessionStore::new(store);
    let store_ref = shared_store.inner.clone();
    let mut builder = Client::builder(app, shared_store).await;

    if use_allow_all {
        warn!("Adding AllowAllPinVerifier for localhost");
        builder = builder.verifier(AllowAllPinVerifier);
    }

    // Add debug configuration
    if let Some(ref debug_config) = config.debug_config
        && let Some(ref session) = debug_config.xdebug_session
    {
        info!("Adding XDEBUG_SESSION header");
        builder = builder.layer_front(XdebugSessionLayer::new(session.clone()));
    }

    // Add proxy configuration
    if config.proxy_config.http_proxy.is_some() {
        info!("Using HTTP_PROXY config");
        builder = builder.proxy(EnvProxy::all("HTTP_PROXY"));
    }

    if config.proxy_config.https_proxy.is_some() {
        info!("Using HTTPS_PROXY config");
        builder = builder.proxy(EnvProxy::all("HTTPS_PROXY"));
    }

    let client = builder.build().context("failed to build client")?;
    Ok((client, store_ref))
}
