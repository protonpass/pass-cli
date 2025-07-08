use crate::store::{AllowAllPinVerifier, AuthenticatorStore, CustomEnv, SerializedEnv};
use crate::utils::ask_for_input;
use anyhow::{Context, bail};
use muon::app::AppVersion;
use muon::client::flow::LoginFlow;
use muon::common::{BoxFut, Sender, SenderLayer};
use muon::env::{Env, EnvId};
use muon::{App, Client, ProtonRequest, ProtonResponse};

const ENVIRONMENT_ENV_VAR: &str = "ENVIRONMENT";
const XDEBUG_SESSION_ENV_VAR: &str = "XDEBUG_SESSION";
const XDEBUG_SESSION_HEADER: &str = "XDEBUG_SESSION";
//const APP_HEADER: &str = "Linux-pass@1.0.0";
const APP_HEADER: &str = "ios-mail@7.1.0";

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

pub async fn authenticate_client(
    client: Client,
    username: &str,
) -> anyhow::Result<AuthenticatedClient> {
    let auth = client.auth();
    let password = ask_for_input("Enter password: ", true)?;
    let client = match auth.login(username, &password).await {
        LoginFlow::Ok(client, _) => client,

        LoginFlow::TwoFactor(client, _) => {
            if client.has_totp() {
                let totp = ask_for_input("Enter TOTP: ", false)?;
                client.totp(&totp).await?
            } else if client.has_fido() {
                unimplemented!()
            } else {
                bail!("no 2FA available");
            }
        }

        LoginFlow::Failed { reason, .. } => {
            bail!("login failed: {reason}, client is staying un-logged.");
        }
    };

    Ok(AuthenticatedClient { client, password })
}

pub async fn get_client() -> anyhow::Result<Client> {
    let app = App::new(APP_HEADER)?;

    let base_dir = crate::utils::get_base_dir().context("failed to get base dir")?;
    let store = AuthenticatorStore::get_from_local(base_dir.clone())
        .await?
        .unwrap_or_else(|| {
            let env = EnvId::from(get_env());
            println!("Using env {env:?}");
            AuthenticatorStore::new_with_path(env, base_dir)
        });

    let mut use_allow_all = false;
    if let EnvId::Custom(ref env) = store.env {
        if let Some(server) = env.servers(&AppVersion::Other).first() {
            let host_name = format!("{}", server.endpoint.host.name());
            if host_name == "localhost" {
                use_allow_all = true;
            }
        }
    }

    let mut builder = Client::builder_async(app, store).await;

    if use_allow_all {
        warn!("Adding AllowAllPinVerifier");
        builder = builder.verifier(AllowAllPinVerifier);
    }

    if let Ok(session) = std::env::var(XDEBUG_SESSION_ENV_VAR) {
        info!("Adding XDEBUG_SESSION header");
        builder = builder.layer_front(XdebugSessionLayer::new(session));
    }

    builder.build().context("failed to build client")
}
