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

use crate::callbacks::AuthEventHandler;
use crate::os::ProdContext;
use crate::store::PassSessionStore;
use aes::Aes256;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use muon::SessionCredentials;
use muon::app::{AppName, AppVersion, SemVer};
use muon::auth::{Auth, Tokens};
use muon::common::sdk::Sdk;
use muon::env::{Env, Environment};
use muon::{GET, Session};
use parking_lot::RwLock;
use pass_domain::aes_gcm::aead::consts::U16;
use pass_domain::aes_gcm::aead::generic_array::GenericArray;
use pass_domain::aes_gcm::aead::{Aead, Payload};
use pass_domain::aes_gcm::{AesGcm, KeyInit};
use std::str::FromStr;
use std::sync::Arc;
use zeroize::{Zeroize, ZeroizeOnDrop};

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

#[derive(Debug, serde::Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SessionPayload {
    #[serde(rename = "keyPassword")]
    pub key_password: String,
}

pub struct WebLoginResult {
    pub credentials: SessionCredentials,
    pub session_payload: SessionPayload,
}

impl SessionPayload {
    pub fn passphrase(&self) -> Vec<u8> {
        self.key_password.as_bytes().to_vec()
    }
}

fn get_account_url_for_env(env: &Environment) -> Result<String> {
    match env {
        Environment::Prod(_) => Ok("https://account.proton.me".to_string()),
        Environment::Atlas(_) => Ok("https://account.proton.black".to_string()),
        Environment::Scientist(s) => {
            // get the scientist name from servers
            let servers = s.servers(&AppVersion::Named {
                name: AppName::from_str("cli-pass").expect("Invalid AppName"),
                version: SemVer::from_str(env!("CARGO_PKG_VERSION")).expect("Invalid SemVer"),
            });
            let host = servers
                .first()
                .map(|srv: &muon::common::Server| format!("{}", srv.host().name()))
                .unwrap_or_default();
            // host is "{product}-api.{name}.proton.black" - extract name
            // For web login URL, we need "account.{name}.proton.black"
            let name = crate::utils::extract_scientist_name(&host);
            Ok(format!("https://account.{}.proton.black", name))
        }
        Environment::Custom(_) => {
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
    session: &Session<ProdContext>,
    sdk: &Sdk,
) -> Result<SessionForkResponse> {
    info!("Fetching session fork...");
    let res = session
        .send_with_sdk(GET!("/auth/sessions/forks"), sdk)
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
    session: &Session<ProdContext>,
    selector: &str,
    event_handler: Arc<dyn AuthEventHandler>,
    sdk: &Sdk,
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
            .send_with_sdk(
                GET!("/auth/sessions/forks/{selector}", selector = selector),
                sdk,
            )
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

fn build_web_login_url(
    env: &Environment,
    user_code: &str,
    encryption_key: &[u8],
) -> Result<String> {
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
    session: &Session<ProdContext>,
    store: Arc<RwLock<PassSessionStore>>,
    event_handler: Arc<dyn AuthEventHandler>,
    sdk: &Sdk,
) -> Result<WebLoginResult> {
    let env = {
        let store_guard = store.read();
        store_guard.env.clone()
    };

    let fork_response = fetch_session_fork(session, sdk)
        .await
        .context("Error fetching session fork")?;

    info!("Session fork created successfully");
    info!("User Code: {}", fork_response.user_code);

    let encryption_key = pass_domain::crypto::generate_encryption_key();
    let url = build_web_login_url(&env, &fork_response.user_code, &encryption_key)
        .context("Error building web login URL")?;

    // Notify event handler of URL generation
    event_handler.on_web_login_url_generated(&url).await?;

    let response = poll_session_fork(session, &fork_response.selector, event_handler.clone(), sdk)
        .await
        .context("Error polling for authentication")?;

    event_handler
        .on_info("Web authentication complete, setting up your account")
        .await?;

    let session_payload = decrypt_payload(&encryption_key, &response.payload)?;
    info!("Payload decrypted correctly");

    let auth = Auth::Internal {
        user_id: response.user_id,
        uid: response.uid,
        tok: Tokens::access(
            response.access_token,
            response.refresh_token,
            response.scopes,
        ),
    };

    let credentials = SessionCredentials::try_from(auth)
        .map_err(|_| anyhow!("Failed to convert web login auth into credentials"))?;

    Ok(WebLoginResult {
        credentials,
        session_payload,
    })
}
