use crate::callbacks::AuthEventHandler;
use crate::store::PassSessionStore;
use aes::Aes256;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use muon::client::{Auth, Tokens};
use muon::env::EnvId;
use muon::store::Store;
use muon::{GET, Session};
use pass::PassSessionKeyType;
use pass_domain::aes_gcm::aead::consts::U16;
use pass_domain::aes_gcm::aead::generic_array::GenericArray;
use pass_domain::aes_gcm::aead::{Aead, Payload};
use pass_domain::aes_gcm::{AesGcm, KeyInit};
use std::sync::Arc;
use tokio::sync::RwLock;

const POLL_INTERVAL_SECONDS: u64 = 10;
const MAX_POLL_ATTEMPTS: u32 = 60; // 60 times every 10 seconds -> 10 minutes total
const MAX_CONSECUTIVE_FAILURES: u32 = 3;
const FAILURE_RETRY_INTERVAL_SECONDS: u64 = 5;
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
pub struct SessionPayload {
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

async fn fetch_session_fork(session: &Session<PassSessionKeyType>) -> Result<SessionForkResponse> {
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
    session: &Session<PassSessionKeyType>,
    selector: &str,
    event_handler: Arc<dyn AuthEventHandler>,
) -> Result<SessionResponse> {
    info!("Starting to poll for authentication...");

    let mut consecutive_failures = 0;

    for attempt in 1..=MAX_POLL_ATTEMPTS {
        info!("Polling attempt {}/{}", attempt, MAX_POLL_ATTEMPTS);

        // Notify event handler of polling progress
        event_handler
            .on_poll_progress(attempt, MAX_POLL_ATTEMPTS)
            .await?;

        let res = match session
            .send(GET!("/auth/sessions/forks/{selector}", selector = selector))
            .await
        {
            Ok(res) => res,
            Err(e) => {
                consecutive_failures += 1;
                warn!(
                    "Polling request failed (consecutive failures: {}): {}",
                    consecutive_failures, e
                );

                if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    return Err(anyhow!(
                        "Failed to poll session fork after {} consecutive failures: {}",
                        MAX_CONSECUTIVE_FAILURES,
                        e
                    ));
                }

                if attempt < MAX_POLL_ATTEMPTS {
                    info!(
                        "Retrying in {} seconds due to failure...",
                        FAILURE_RETRY_INTERVAL_SECONDS
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(
                        FAILURE_RETRY_INTERVAL_SECONDS,
                    ))
                    .await;
                }
                continue;
            }
        };

        // Request succeeded, reset failure counter
        consecutive_failures = 0;

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

pub async fn perform_web_login(
    session: Session<PassSessionKeyType>,
    store: Arc<RwLock<PassSessionStore>>,
    event_handler: Arc<dyn AuthEventHandler>,
) -> Result<SessionPayload> {
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

    // Notify event handler of URL generation
    event_handler.on_web_login_url_generated(&url).await?;

    let response = poll_session_fork(&session, &fork_response.selector, event_handler.clone())
        .await
        .context("Error polling for authentication")?;

    event_handler
        .on_info("Web authentication complete, setting up your account")
        .await?;

    let session_payload = decrypt_payload(&encryption_key, &response.payload)?;
    info!("Payload decrypted correctly");

    // Store the auth
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

        let mut store_guard = store.write().await;
        store_guard
            .set_auth(&(), auth)
            .await
            .context("Error setting auth")?;
        // Set account type for regular user login
        store_guard.set_account_type(pass_domain::AccountType::User);
    }

    Ok(session_payload)
}
