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

use crate::os::ProdContext;
use anyhow::{Context, Result, anyhow, bail};
use base64::Engine;
use muon::POST;
use muon::SessionCredentials;
use muon::auth::{Auth, Tokens};
use zeroize::Zeroizing;

const TOKEN_PREFIX: &str = "pst_";
const TOKEN_LENGTH_WITHOUT_PREFIX: usize = 64;
const TOKEN_SEPARATOR: &str = "::";

#[derive(Debug, serde::Deserialize)]
struct PersonalAccessTokenSessionResponse {
    #[serde(rename = "Session")]
    session: PersonalAccessTokenSession,
}

#[derive(Debug, serde::Deserialize)]
struct PersonalAccessTokenSession {
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
struct PersonalAccessTokenLoginRequest {
    #[serde(rename = "Token")]
    token: String,
}

pub struct ParsedPersonalAccessTokenToken {
    pub token: String,
    pub personal_access_token_key: Zeroizing<Vec<u8>>,
}

pub struct PersonalAccessTokenLoginResult {
    pub credentials: SessionCredentials,
    pub personal_access_token_key: Zeroizing<Vec<u8>>,
}

pub fn parse_personal_access_token_token(
    token_string: &str,
) -> Result<ParsedPersonalAccessTokenToken> {
    // Split by ::
    let parts: Vec<&str> = token_string.split(TOKEN_SEPARATOR).collect();
    if parts.len() != 2 {
        bail!("Invalid personal access token token format. Expected format: pst_<token>::<key>");
    }

    let token = parts[0];
    let key_b64 = parts[1];

    // Validate token format
    if !token.starts_with(TOKEN_PREFIX) {
        bail!(
            "Personal access token token must start with '{}'",
            TOKEN_PREFIX
        );
    }

    let token_without_prefix = &token[TOKEN_PREFIX.len()..];
    if token_without_prefix.len() != TOKEN_LENGTH_WITHOUT_PREFIX {
        bail!(
            "Personal access token token must have exactly {} characters after '{}' prefix",
            TOKEN_LENGTH_WITHOUT_PREFIX,
            TOKEN_PREFIX
        );
    }

    // Decode the personal access token key (base64 URL-safe)
    let personal_access_token_key = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(key_b64)
        .context("Failed to decode personal access token key. Must be base64-urlsafe encoded")?;

    Ok(ParsedPersonalAccessTokenToken {
        token: token.to_string(),
        personal_access_token_key: Zeroizing::new(personal_access_token_key),
    })
}

async fn create_personal_access_token_session(
    session: &muon::Session<ProdContext>,
    token: &str,
) -> Result<PersonalAccessTokenSessionResponse> {
    info!("Creating personal access token session...");

    let request = PersonalAccessTokenLoginRequest {
        token: token.to_string(),
    };

    let res = session
        .send(
            POST!("/account/v4/personal-access-token/session")
                .body_json(&request)
                .context("Failed to create personal access token session request")?,
        )
        .await
        .context("Error requesting personal access token session")?;

    if !res.status().is_success() {
        debug_response(&res);
        return Err(anyhow!(
            "Bad response when creating Personal Access Token session: {:?}",
            res.status()
        ));
    }

    let session_response: PersonalAccessTokenSessionResponse = res
        .body_json()
        .context("Error parsing personal access token session response")?;

    Ok(session_response)
}

pub async fn perform_personal_access_token_login(
    session: &muon::Session<ProdContext>,
    token_string: &str,
) -> Result<PersonalAccessTokenLoginResult> {
    // Parse the token
    let parsed = parse_personal_access_token_token(token_string)
        .context("Failed to parse personal access token token")?;

    info!("Personal access token token parsed successfully");

    // Request personal access token session
    let response = create_personal_access_token_session(session, &parsed.token)
        .await
        .context("Error creating personal access token session")?;

    info!("Personal access token session created successfully");

    let auth = Auth::Internal {
        user_id: response.session.session_uid.clone(),
        uid: response.session.session_uid.clone(),
        tok: Tokens::access(
            response.session.access_token,
            response.session.refresh_token,
            response.session.scopes,
        ),
    };

    let credentials = SessionCredentials::try_from(auth)
        .map_err(|_| anyhow!("Failed to convert personal access token auth into credentials"))?;

    Ok(PersonalAccessTokenLoginResult {
        credentials,
        personal_access_token_key: parsed.personal_access_token_key,
    })
}

fn debug_response(res: &muon::http::HttpRes) {
    match res.body_str() {
        Ok(body) => {
            debug!("{body}");
        }
        Err(e) => {
            error!("Cannot get HttpRes body_str: {e:#}");
        }
    }
}
