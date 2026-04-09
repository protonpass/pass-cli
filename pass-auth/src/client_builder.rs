use crate::config::ClientConfig;
use crate::os::{ProdClient, ProdOs, TokioExecutor};
use crate::storage::SessionStorage;
use crate::store::{
    CustomEnv, GetStoreError, PassSessionStore, SerializedEnv, SharedPassSessionStore,
};
use anyhow::{Context, anyhow};
use muon::app::App;
use muon::client::builder::Hyper;
use muon::common::{EnvProxy, Proxy};
use muon::env::{Env, Environment};
use pass_domain::LocalKeyProvider;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use std::sync::{Arc, RwLock};

pub const ENVIRONMENT_ENV_VAR: &str = "PROTON_PASS_ENVIRONMENT";
const XDEBUG_SESSION_HEADER: &str = "XDEBUG_SESSION";
const APP_NAME: &str = "cli-pass";

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

fn store_using_current_env(store_env: &Environment, current_env: &Environment) -> bool {
    match (store_env, current_env) {
        (Environment::Prod(_), Environment::Prod(_)) => true,
        (Environment::Custom(_), Environment::Custom(_)) => true,
        (Environment::Atlas(_), Environment::Atlas(_)) => true,
        (Environment::Scientist(s1), Environment::Scientist(s2)) => {
            // Compare by serializing through SerializedEnv
            let s1_serialized = SerializedEnv::from(Environment::Scientist(s1.clone()));
            let s2_serialized = SerializedEnv::from(Environment::Scientist(s2.clone()));
            matches!(
                (s1_serialized, s2_serialized),
                (SerializedEnv::Atlas(Some(a)), SerializedEnv::Atlas(Some(b))) if a == b
            )
        }
        _ => false,
    }
}

pub async fn create_client(
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    config: &ClientConfig,
) -> anyhow::Result<(ProdClient, Arc<RwLock<PassSessionStore>>)> {
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

    let serialized_env = get_env(config);
    debug!("Serialized env: {serialized_env:?}");
    let current_env = Environment::from(serialized_env);
    debug!("Current env: {current_env:?}");

    let servers = current_env.servers(app.app_version());
    debug!("Servers: {servers:?}");

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

    // Build the client
    let shared_store = SharedPassSessionStore::new(store);
    let store_ref = shared_store.inner.clone();

    // Proxy must be configured before with_persistence due to typestate constraints
    let mut transport_builder = muon::Client::builder_with_transport::<Hyper>(app, current_env)
        .with_operating_system(ProdOs::default(), ChaCha20Rng::from_os_rng())
        .with_multi_thread_executor(TokioExecutor);

    if config.proxy_config.http_proxy.is_some() {
        info!("Using HTTP_PROXY config");
        transport_builder = transport_builder.proxy(Proxy::Env(EnvProxy::all("HTTP_PROXY")));
    }

    if config.proxy_config.https_proxy.is_some() {
        info!("Using HTTPS_PROXY config");
        transport_builder = transport_builder.proxy(Proxy::Env(EnvProxy::all("HTTPS_PROXY")));
    }

    let mut builder = transport_builder.with_persistence(shared_store);

    // Add XDEBUG_SESSION header if configured
    if let Some(ref debug_config) = config.debug_config
        && let Some(ref session) = debug_config.xdebug_session
    {
        info!("Adding XDEBUG_SESSION header");
        builder = builder.with_default_headers((XDEBUG_SESSION_HEADER, session.clone()));
    }

    let client = builder.build().context("failed to build client")?;
    Ok((client, store_ref))
}
