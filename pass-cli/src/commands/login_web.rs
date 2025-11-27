use crate::features::CliClientFeatures;
use crate::store::PassSessionStore;
use crate::{client, utils};
use aes::Aes256;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use muon::client::{Auth, Tokens};
use muon::env::EnvId;
use muon::store::Store;
use muon::{GET, Session};
use pass::{Client, FirstTimeSetupKey, PassClient};
use pass_domain::aes_gcm::aead::consts::U16;
use pass_domain::aes_gcm::aead::generic_array::GenericArray;
use pass_domain::aes_gcm::aead::{Aead, Payload};
use pass_domain::aes_gcm::{AesGcm, KeyInit};
use std::sync::Arc;
use tokio::sync::RwLock;

const POLL_INTERVAL_SECONDS: u64 = 10;
const MAX_POLL_ATTEMPTS: u32 = 60; // 60 times every 10 seconds -> 10 minutes total
const CHILD_CLIENT_ID: &str = "cli-pass";

#[derive(Debug, serde::Deserialize)]
struct SessionForkResponse {
    #[serde(rename = "UserCode")]
    user_code: String,
    #[serde(rename = "Selector")]
    selector: String,
}

#[derive(Debug, serde::Deserialize)]
struct SessionResponse {
    #[serde(rename = "Payload")]
    pub payload: String,
    #[serde(rename = "Scopes")]
    pub scopes: Vec<String>,
    #[serde(rename = "UID")]
    pub uid: String,
    #[serde(rename = "UserID")]
    pub user_id: String,
    #[serde(rename = "AccessToken")]
    pub access_token: String,
    #[serde(rename = "RefreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, serde::Deserialize)]
struct SessionPayload {
    #[serde(rename = "keyPassword")]
    pub key_password: String,
}

impl SessionPayload {
    pub fn passphrase(&self) -> Vec<u8> {
        self.key_password.as_bytes().to_vec()
    }
}

fn get_account_url_for_env(env: &EnvId) -> Result<String> {
    match env {
        EnvId::Prod => Ok("https://account.proton.me".to_string()),
        EnvId::Atlas(None) => Ok("https://account.proton.black".to_string()),
        EnvId::Atlas(Some(atlas_env)) => Ok(format!("https://account.{}.proton.black", atlas_env)),
        EnvId::Custom(_) => {
            bail!("Web login is not supported for custom environments")
        }
    }
}

fn decrypt_payload(encryption_key: &[u8], payload: &str) -> Result<SessionPayload> {
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .context("Error decoding payload")?;

    // Check that the ciphertext is at least large enough to contain the nonce.
    if ciphertext.len() < 16 {
        bail!("Payload is too short");
    }

    // Extract nonce and actual ciphertext.
    let (nonce_bytes, cipherdata) = ciphertext.split_at(16);

    // Use AES-256-GCM with a 16-byte nonce
    type Aes256GcmWith16ByteNonce = AesGcm<Aes256, U16>;
    let cipher = Aes256GcmWith16ByteNonce::new(GenericArray::from_slice(encryption_key));
    let nonce = GenericArray::<u8, U16>::from_slice(nonce_bytes);

    let payload = Payload {
        msg: cipherdata,
        aad: &[],
    };
    let decrypted = cipher
        .decrypt(nonce, payload)
        .map_err(|e| anyhow!("Error decrypting payload: {e}"))?;
    let parsed: SessionPayload =
        serde_json::from_slice(&decrypted).context("Error parsing payload")?;

    Ok(parsed)
}

async fn fetch_session_fork(
    session: &Session<pass::PassSessionKeyType>,
) -> Result<SessionForkResponse> {
    info!("Fetching session fork...");
    let res = session
        .send(GET!("/auth/sessions/forks"))
        .await
        .context("Error requesting session fork")?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP Status: {:?}", res.status()));
    }

    let fork_response: SessionForkResponse = res
        .body_json()
        .context("Error parsing session fork response")?;

    Ok(fork_response)
}

async fn poll_session_fork(
    session: &Session<pass::PassSessionKeyType>,
    selector: &str,
) -> Result<SessionResponse> {
    info!("Starting to poll for authentication...");

    for attempt in 1..=MAX_POLL_ATTEMPTS {
        info!("Polling attempt {}/{}", attempt, MAX_POLL_ATTEMPTS);

        let res = session
            .send(GET!("/auth/sessions/forks/{selector}", selector = selector))
            .await
            .context("Error polling session fork")?;

        if res.status().is_success() {
            info!("Authentication successful!");
            let response_json: SessionResponse =
                res.body_json().context("Error parsing polling response")?;
            return Ok(response_json);
        }

        if attempt < MAX_POLL_ATTEMPTS {
            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECONDS)).await;
        }
    }

    Err(anyhow!(
        "Authentication timed out after {} seconds",
        POLL_INTERVAL_SECONDS * MAX_POLL_ATTEMPTS as u64
    ))
}

fn build_web_login_url(env: &EnvId, user_code: &str, encryption_key: &[u8]) -> Result<String> {
    let account_url = get_account_url_for_env(env)?;

    let encoded_key = base64::engine::general_purpose::STANDARD.encode(encryption_key);
    let payload = format!("0:{}:{}:{}", user_code, encoded_key, CHILD_CLIENT_ID);

    let encoded_payload = urlencoding::encode(&payload);

    let url = format!(
        "{}/desktop/login?app=pass#payload={}",
        account_url, encoded_payload
    );

    Ok(url)
}

async fn create_new_client(
    client_features: Arc<CliClientFeatures>,
) -> Result<(PassClient, Arc<RwLock<PassSessionStore>>)> {
    let base_dir = utils::get_base_dir().context("Error getting base dir")?;

    let (client, store) = client::get_client(base_dir.clone(), client_features.clone())
        .await
        .context("Error getting client")?;

    Ok((PassClient::new(client, client_features), store))
}

pub async fn run(
    client: Client,
    client_features: Arc<CliClientFeatures>,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    let session = client.get_session(()).await;
    if let Some(session) = session
        && session.is_authenticated().await
    {
        eprintln!("Client is already authenticated. Log out if you want to log in again");
        return Ok(());
    }

    let session = client
        .new_session_without_credentials(())
        .await
        .context("Error creating session")?;

    let env = {
        let store_guard = store.read().await;
        store_guard.env.clone()
    };

    let fork_response = fetch_session_fork(&session)
        .await
        .context("Error fetching session fork")?;

    info!("Session fork created successfully");
    info!("User Code: {}", fork_response.user_code);

    let encryption_key = pass_domain::crypto::generate_encryption_key();
    let url = build_web_login_url(&env, &fork_response.user_code, &encryption_key)
        .context("Error building web login URL")?;

    println!("\nPlease open the following URL in your browser to complete authentication:");
    println!("\n{}\n", url);
    println!("Waiting for authentication to complete...");

    let response = poll_session_fork(&session, &fork_response.selector)
        .await
        .context("Error polling for authentication")?;
    println!("Web authentication complete, setting up your account");

    let session_payload = decrypt_payload(&encryption_key, &response.payload)?;
    info!("Payload decrypted correctly");

    {
        let auth = Auth::Internal {
            user_id: response.user_id,
            uid: response.uid,
            tok: Tokens::access(
                response.access_token,
                response.refresh_token,
                response.scopes,
            ),
        };

        store
            .write()
            .await
            .set_auth(&(), auth)
            .await
            .context("Error setting auth")?;
    }

    // HACK: Create a new client to make sure we're using the right store, as the old one sometimes
    // has credentials locally-cached and doesn't work well
    let (pass_client, store) = create_new_client(client_features).await?;

    // Check if it needs extra password
    let needs_extra_password = {
        let store_guard = store.read().await;
        store_guard.needs_extra_password().await
    };

    if needs_extra_password {
        info!("Account needs Pass extra password");

        let session = pass_client.get_session().await?;
        crate::extra_password::handle_extra_password(&session).await?;
    }

    // Attempt to retrieve client info to make sure we can actually perform a request
    let user_info = pass_client.get_info().await.context("Error getting info")?;
    println!("Login performed by {}", user_info.user.email);

    let passphrase = session_payload.passphrase();
    super::login::after_login(
        pass_client,
        FirstTimeSetupKey::Passphrase(passphrase),
        store,
    )
    .await?;

    println!("Successfully logged in as {}", user_info.user.email);
    Ok(())
}
