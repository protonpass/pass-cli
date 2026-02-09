use crate::features::CliClientFeatures;
use crate::store::{
    AllowAllPinVerifier, CustomEnv, GetStoreError, PassSessionStore, SerializedEnv,
    SharedPassSessionStore,
};
use crate::utils::ask_for_input;
use anyhow::{Context, bail};
use muon::app::AppVersion;
use muon::client::flow::LoginFlow;
use muon::common::{BoxFut, EnvProxy, Sender, SenderLayer};
use muon::env::{Env, EnvId};
use muon::{App, GET, ProtonRequest, ProtonResponse, Session};
use pass::{Client, PassSessionKeyType};
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

const ENVIRONMENT_ENV_VAR: &str = "PROTON_PASS_ENVIRONMENT";
const XDEBUG_SESSION_ENV_VAR: &str = "XDEBUG_SESSION";
const XDEBUG_SESSION_HEADER: &str = "XDEBUG_SESSION";
const APP_NAME: &str = "cli-pass";

const PASSWORD_ENV_VAR: &str = "PROTON_PASS_PASSWORD";
const PASSWORD_FILE_ENV_VAR: &str = "PROTON_PASS_PASSWORD_FILE";
const EXTRA_PASSWORD_ENV_VAR: &str = "PROTON_PASS_EXTRA_PASSWORD";
const EXTRA_PASSWORD_FILE_ENV_VAR: &str = "PROTON_PASS_EXTRA_PASSWORD_FILE";
const TOTP_ENV_VAR: &str = "PROTON_PASS_TOTP";
const TOTP_FILE_ENV_VAR: &str = "PROTON_PASS_TOTP_FILE";
const USERNAME_ENV_VAR: &str = "PROTON_PASS_USERNAME";
const USERNAME_FILE_ENV_VAR: &str = "PROTON_PASS_USERNAME_FILE";
const APP_HEADER_ENV_VAR: &str = "PROTON_PASS_APP_HEADER";

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

fn get_env() -> SerializedEnv {
    match std::env::var(ENVIRONMENT_ENV_VAR) {
        Ok(v) => {
            if v == "atlas" {
                SerializedEnv::Atlas(None)
            } else if v == "localhost" {
                SerializedEnv::Custom(CustomEnv::Localhost)
            } else if v.starts_with("http") {
                SerializedEnv::Custom(CustomEnv::CustomUrl(v))
            } else {
                SerializedEnv::Atlas(Some(v))
            }
        }
        Err(_) => SerializedEnv::Prod,
    }
}

pub struct AuthenticatedClient {
    pub client: Client,
    pub password: String,
}

pub fn get_value(
    env_var: &str,
    file_env_var: &str,
    prompt: &str,
    secure: bool,
) -> anyhow::Result<String> {
    match std::env::var(env_var) {
        Ok(v) => Ok(v),
        Err(_) => match std::env::var(file_env_var) {
            Ok(v) => {
                let mut f = std::fs::File::open(v).context("Error opening file")?;
                let mut buff = String::new();
                f.read_to_string(&mut buff).context("Error reading file")?;
                Ok(buff.trim().to_string())
            }
            Err(_) => ask_for_input(prompt, secure),
        },
    }
}

fn get_password() -> anyhow::Result<String> {
    get_value(
        PASSWORD_ENV_VAR,
        PASSWORD_FILE_ENV_VAR,
        "Enter password: ",
        true,
    )
}

pub fn get_extra_password() -> anyhow::Result<String> {
    get_value(
        EXTRA_PASSWORD_ENV_VAR,
        EXTRA_PASSWORD_FILE_ENV_VAR,
        "Enter Pass extra password: ",
        true,
    )
}

fn get_totp() -> anyhow::Result<String> {
    get_value(TOTP_ENV_VAR, TOTP_FILE_ENV_VAR, "Enter TOTP: ", false)
}

pub fn get_username() -> anyhow::Result<String> {
    get_value(
        USERNAME_ENV_VAR,
        USERNAME_FILE_ENV_VAR,
        "Enter username: ",
        false,
    )
}

pub async fn authenticate_client(
    client: Client,
    username: &str,
    store: Arc<RwLock<PassSessionStore>>,
) -> anyhow::Result<AuthenticatedClient> {
    let session = client
        .new_session_without_credentials(())
        .await
        .context("Error creating session")?;
    let auth = session.auth();
    let password = get_password()?;
    let session = match auth.login(username, &password).await {
        LoginFlow::Ok(session, _) => session,

        LoginFlow::TwoFactor(session, _) => {
            let has_totp = session.has_totp();
            let has_fido = session.fido_details().is_some();

            match (has_totp, has_fido) {
                (true, _) => {
                    if has_fido {
                        println!(
                            "Your account has many 2FA methods available. Using TOTP. If you want to use others, use 'pass-cli login' and login via web"
                        );
                    }
                    let totp = get_totp()?;
                    session.totp(&totp).await?
                }
                (false, true) => {
                    println!(
                        "Your account cannot login interactively. Use 'pass-cli login' and login via web"
                    );
                    std::process::exit(1);
                }
                (false, false) => bail!("no 2FA available"),
            }
        }

        LoginFlow::Failed { reason, .. } => {
            eprintln!("Authentication failed: {reason}");
            eprintln!("Make sure the password you entered is the right one.");
            std::process::exit(1);
        }
    };

    // Check if it needs extra password
    let needs_extra_password = {
        let store_guard = store.read().await;
        store_guard.needs_extra_password().await
    };

    if needs_extra_password {
        info!("Account needs Pass extra password");
        crate::extra_password::handle_extra_password(&session).await?;
    }

    init_session(&session)
        .await
        .context("Error initializing session")?;
    Ok(AuthenticatedClient { client, password })
}

pub async fn init_session(session: &Session<PassSessionKeyType>) -> anyhow::Result<()> {
    session
        .send(GET!("/tests/ping"))
        .await
        .context("Error initializing session")?;
    Ok(())
}

fn default_app_header() -> String {
    format!("{}@{}", APP_NAME, env!("CARGO_PKG_VERSION"))
}

fn get_app_header() -> String {
    std::env::var(APP_HEADER_ENV_VAR).unwrap_or_else(|_| default_app_header())
}

fn store_using_current_env(env_id: &EnvId) -> bool {
    let env = EnvId::from(get_env());
    match env {
        EnvId::Prod => matches!(env_id, EnvId::Prod),
        EnvId::Custom(_) => matches!(env_id, EnvId::Custom(_)),
        EnvId::Atlas(ref current_atlas_env) => match env_id {
            EnvId::Atlas(store_atlas_env) => store_atlas_env == current_atlas_env,
            _ => false,
        },
    }
}

pub async fn get_client(
    base_dir: PathBuf,
    client_features: Arc<CliClientFeatures>,
) -> anyhow::Result<(Client, Arc<RwLock<PassSessionStore>>)> {
    // Check key_provider can be used
    client_features
        .key_provider
        .get_key()
        .await
        .context("Error accessing key provider")?;

    let app = App::new(get_app_header()).context("failed to create app")?;
    let key_provider = client_features.key_provider.clone();

    let store = match PassSessionStore::get_from_local(base_dir.clone(), key_provider.clone()).await
    {
        Ok(store) => store,
        Err(e) => {
            return match e {
                GetStoreError::CannotDecrypt(e) => Err(anyhow::anyhow!(
                    "Error decrypting local session({e:#}). Make sure you have not changed your key provider / removed your local key, or try to logout and log in again"
                )),
                GetStoreError::Other(e) => {
                    Err(anyhow::anyhow!("Error loading local session: {e:#}"))
                }
            };
        }
    };

    let store = store.unwrap_or_else(|| {
        let env = EnvId::from(get_env());
        debug!("Using env {env:?}");
        PassSessionStore::new_with_path(env, base_dir, key_provider)
    });

    let mut use_allow_all = false;
    if let EnvId::Custom(ref env) = store.env
        && let Some(server) = env.servers(&AppVersion::Other).first()
    {
        let host_name = format!("{}", server.endpoint.host.name());
        if host_name == "localhost" {
            use_allow_all = true;
        }
    }

    if !store_using_current_env(&store.env) {
        eprintln!("ENVIRONMENT has switched! Logging you out. Please log back in again");
        crate::commands::logout::force_logout().await?;
        std::process::exit(1);
    }

    let shared_store = SharedPassSessionStore::new(store);
    let store_ref = shared_store.inner.clone();
    let mut builder = Client::builder(app, shared_store).await;

    if use_allow_all {
        warn!("Adding AllowAllPinVerifier");
        builder = builder.verifier(AllowAllPinVerifier);
    }

    if let Ok(session) = std::env::var(XDEBUG_SESSION_ENV_VAR) {
        info!("Adding XDEBUG_SESSION header");
        builder = builder.layer_front(XdebugSessionLayer::new(session));
    }

    if std::env::var("HTTP_PROXY").is_ok() {
        eprintln!("Using HTTP_PROXY config");
        builder = builder.proxy(EnvProxy::all("HTTP_PROXY"));
    }
    if std::env::var("HTTPS_PROXY").is_ok() {
        eprintln!("Using HTTPS_PROXY config");
        builder = builder.proxy(EnvProxy::all("HTTPS_PROXY"));
    }

    let client = builder.build().context("failed to build client")?;
    Ok((client, store_ref))
}
