use crate::store::PassSessionStore;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use muon::POST;
use muon::client::{Auth, Tokens};
use muon::store::Store;
use pass::PassSessionKeyType;
use std::sync::Arc;
use tokio::sync::RwLock;

const TOKEN_PREFIX: &str = "ppsa_";
const TOKEN_LENGTH_WITHOUT_PREFIX: usize = 64;
const TOKEN_SEPARATOR: &str = "::";

#[derive(Debug, serde::Deserialize)]
struct ServiceAccountSessionResponse {
    #[serde(rename = "Session")]
    session: ServiceAccountSession,
}

#[derive(Debug, serde::Deserialize)]
struct ServiceAccountSession {
    #[serde(rename = "SessionUID")]
    session_uid: String,
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "RefreshToken")]
    refresh_token: String,
    #[serde(rename = "AccessExpirationTime")]
    #[allow(dead_code)]
    access_expiration_time: Option<i64>,
    #[serde(rename = "RefreshExpirationTime")]
    #[allow(dead_code)]
    refresh_expiration_time: Option<i64>,
    #[serde(rename = "Scopes")]
    scopes: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct ServiceAccountLoginRequest {
    #[serde(rename = "Token")]
    token: String,
}

pub struct ParsedServiceAccountToken {
    pub token: String,
    pub service_account_key: Vec<u8>,
}

pub fn parse_service_account_token(token_string: &str) -> Result<ParsedServiceAccountToken> {
    // Split by ::
    let parts: Vec<&str> = token_string.split(TOKEN_SEPARATOR).collect();
    if parts.len() != 2 {
        bail!("Invalid service account token format. Expected format: ppsa_<token>::<key>");
    }

    let token = parts[0];
    let key_b64 = parts[1];

    // Validate token format
    if !token.starts_with(TOKEN_PREFIX) {
        bail!("Service account token must start with '{}'", TOKEN_PREFIX);
    }

    let token_without_prefix = &token[TOKEN_PREFIX.len()..];
    if token_without_prefix.len() != TOKEN_LENGTH_WITHOUT_PREFIX {
        bail!(
            "Service account token must have exactly {} characters after '{}' prefix",
            TOKEN_LENGTH_WITHOUT_PREFIX,
            TOKEN_PREFIX
        );
    }

    // Decode the service account key (base64 URL-safe)
    let service_account_key = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(key_b64)
        .context("Failed to decode service account key. Must be base64-urlsafe encoded")?;

    Ok(ParsedServiceAccountToken {
        token: token.to_string(),
        service_account_key,
    })
}

async fn create_service_account_session(
    session: &muon::Session<PassSessionKeyType>,
    token: &str,
) -> Result<ServiceAccountSessionResponse> {
    info!("Creating service account session...");

    let request = ServiceAccountLoginRequest {
        token: token.to_string(),
    };

    let res = session
        .send(
            POST!("/pass/v1/service_account/session")
                .body_json(&request)
                .context("Failed to create service account session request")?,
        )
        .await
        .context("Error requesting service account session")?;

    if !res.status().is_success() {
        return Err(anyhow!("HTTP Status: {:?}", res.status()));
    }

    let session_response: ServiceAccountSessionResponse = res
        .body_json()
        .context("Error parsing service account session response")?;

    Ok(session_response)
}

pub async fn perform_service_account_login(
    session: muon::Session<PassSessionKeyType>,
    store: Arc<RwLock<PassSessionStore>>,
    token_string: &str,
) -> Result<Vec<u8>> {
    // Parse the token
    let parsed = parse_service_account_token(token_string)
        .context("Failed to parse service account token")?;

    info!("Service account token parsed successfully");

    // Request service account session
    let response = create_service_account_session(&session, &parsed.token)
        .await
        .context("Error creating service account session")?;

    info!("Service account session created successfully");

    // Set authentication
    {
        let auth = Auth::Internal {
            user_id: response.session.session_uid.clone(),
            uid: response.session.session_uid.clone(),
            tok: Tokens::access(
                response.session.access_token,
                response.session.refresh_token,
                response.session.scopes,
            ),
        };

        let mut store_guard = store.write().await;
        // Set account type for service account login
        store_guard.set_account_type(pass_domain::AccountType::ServiceAccount);
        store_guard
            .set_auth(&(), auth)
            .await
            .context("Error setting auth")?;
    }

    Ok(parsed.service_account_key)
}
