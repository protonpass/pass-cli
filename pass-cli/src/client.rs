use crate::features::CliClientFeatures;
use crate::store::{
    AllowAllPinVerifier, AuthenticatorStore, CustomEnv, GetStoreError, SerializedEnv,
};
use crate::utils::ask_for_input;
use anyhow::{Context, anyhow, bail};
use base64urlsafedata::Base64UrlSafeData;
use muon::app::AppVersion;
use muon::client::flow::{LoginFlow, LoginTwoFactorFlow};
use muon::common::{BoxFut, Sender, SenderLayer};
use muon::env::{Env, EnvId};
use muon::rest::auth::v4::fido2;
use muon::util::ByteSliceExt;
use muon::{App, Client, ProtonRequest, ProtonResponse};
use std::io::Read;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use webauthn_authenticator_rs::prelude::{RequestChallengeResponse, Url};
use webauthn_rs_proto::{
    AllowCredentials, AuthenticatorTransport, RequestAuthenticationExtensions,
    UserVerificationPolicy,
};

const ENVIRONMENT_ENV_VAR: &str = "ENVIRONMENT";
const XDEBUG_SESSION_ENV_VAR: &str = "XDEBUG_SESSION";
const XDEBUG_SESSION_HEADER: &str = "XDEBUG_SESSION";
const APP_NAME: &str = "cli-pass";

const PASSWORD_ENV_VAR: &str = "PROTON_PASS_PASSWORD";
const PASSWORD_FILE_ENV_VAR: &str = "PROTON_PASS_PASSWORD_FILE";
const TOTP_ENV_VAR: &str = "PROTON_PASS_TOTP";
const TOTP_FILE_ENV_VAR: &str = "PROTON_PASS_TOTP_FILE";
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

fn get_value(
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

fn get_totp() -> anyhow::Result<String> {
    get_value(TOTP_ENV_VAR, TOTP_FILE_ENV_VAR, "Enter TOTP: ", false)
}

pub async fn authenticate_client(
    client: Client,
    username: &str,
) -> anyhow::Result<AuthenticatedClient> {
    let auth = client.auth();
    let password = get_password()?;
    let client = match auth.login(username, &password).await {
        LoginFlow::Ok(client, _) => client,

        LoginFlow::TwoFactor(client, _) => {
            if client.has_totp() {
                let totp = get_totp()?;
                client.totp(&totp).await?
            } else if client.fido_details().is_some() {
                handle_fido(client).await?
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

async fn handle_fido(client: LoginTwoFactorFlow) -> anyhow::Result<Client> {
    let details = client
        .fido_details()
        .ok_or_else(|| anyhow!("Missing fido details"))?;
    let options = match details.authentication_options {
        Some(ref opts) => opts.clone(),
        None => return Err(anyhow!("No authentication options provided")),
    };

    let allow_credentials = match options.public_key.allow_credentials {
        Some(ref creds) => !creds.is_empty(),
        None => false,
    };

    if !allow_credentials {
        return Err(anyhow!("No Fido2 authentication options not available"));
    }

    let authenticator = webauthn_authenticator_rs::mozilla::MozillaAuthenticator::new();
    let mut authenticator = webauthn_authenticator_rs::WebauthnAuthenticator::new(authenticator);

    let options_cloned = options.clone();
    let pk = options_cloned.public_key;
    let url = if let Some(ref rp_id) = pk.rp_id {
        format!("https://{}", rp_id)
    } else {
        return Err(anyhow!("Missing rp_id in FIDO2"));
    };
    let url = Url::parse(&url).context("Invalid rp_id URL in FIDO2 request")?;

    eprintln!("Starting FIDO2 authentication");
    let res = authenticator
        .do_authentication(
            url,
            RequestChallengeResponse {
                public_key: webauthn_rs_proto::PublicKeyCredentialRequestOptions {
                    challenge: Base64UrlSafeData::from(pk.challenge),
                    timeout: None,
                    rp_id: pk
                        .rp_id
                        .ok_or_else(|| anyhow!("Missing or empty rp_id in FIDO2"))?,
                    allow_credentials: pk
                        .allow_credentials
                        .unwrap_or_default()
                        .into_iter()
                        .map(|k| AllowCredentials {
                            type_: k.credential_type,
                            id: Base64UrlSafeData::from(k.id),
                            transports: k.transports.map(|transports| {
                                transports
                                    .into_iter()
                                    .filter_map(|t| AuthenticatorTransport::from_str(&t).ok())
                                    .collect()
                            }),
                        })
                        .collect(),
                    user_verification: match pk.user_verification {
                        Some(uv) => match uv.as_str() {
                            "required" => UserVerificationPolicy::Required,
                            "discouraged" => UserVerificationPolicy::Discouraged_DO_NOT_USE,
                            "preferred" => UserVerificationPolicy::Preferred,
                            _ => UserVerificationPolicy::Preferred,
                        },
                        None => UserVerificationPolicy::Preferred,
                    },
                    hints: None,
                    extensions: pk.extensions.map(|e| RequestAuthenticationExtensions {
                        appid: e.app_id,
                        uvm: e.uvm,
                        hmac_get_secret: None,
                    }),
                },
                mediation: None,
            },
        )
        .context("Failure in FIDO2 authentication")?;

    let request = fido2::Request {
        authentication_options: options,
        client_data: res.response.client_data_json.as_b64(),
        authenticator_data: res.response.authenticator_data.as_b64(),
        signature: res.response.signature.as_b64(),
        credential_id: res.get_credential_id().to_vec(),
    };

    let authenticated_client = client
        .fido(request)
        .await
        .context("Error sending FIDO2 response for login")?;
    Ok(authenticated_client)
}

fn default_app_header() -> String {
    format!("{}@{}", APP_NAME, env!("CARGO_PKG_VERSION"))
}

fn get_app_header() -> String {
    std::env::var(APP_HEADER_ENV_VAR).unwrap_or_else(|_| default_app_header())
}

pub async fn get_client(
    base_dir: PathBuf,
    client_features: Arc<CliClientFeatures>,
) -> anyhow::Result<Client> {
    let app = App::new(get_app_header()).context("failed to create app")?;
    let key_provider = client_features.key_provider.clone();

    let store = match AuthenticatorStore::get_from_local(base_dir.clone(), key_provider.clone())
        .await
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
        AuthenticatorStore::new_with_path(env, base_dir, key_provider)
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
