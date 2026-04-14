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

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use base64::Engine;
use muon::POST;
use pass_domain::PersonalAccessTokenId;

#[derive(Clone, Debug, serde::Serialize)]
struct RenewPersonalAccessTokenRequest {
    #[serde(rename = "ExpireTime")]
    expire_time: i64,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct RenewPersonalAccessTokenResponseData {
    #[serde(rename = "PersonalAccessToken")]
    personal_access_token: RenewedPersonalAccessTokenData,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct RenewedPersonalAccessTokenData {
    #[serde(rename = "Token")]
    token: String,
}

pub struct RenewPersonalAccessTokenResponse {
    pub token: String,
    pub env_var: String,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn renew_personal_access_token(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
        expiration_time: i64,
    ) -> Result<RenewPersonalAccessTokenResponse> {
        self.personal_access_token_operation_guard()?;
        info!("Renewing personal access token: {personal_access_token_id}");

        let raw_pat_key = self
            .get_personal_access_token_key(personal_access_token_id)
            .await
            .context("Failed to get personal access token key")?;

        let request = RenewPersonalAccessTokenRequest {
            expire_time: expiration_time,
        };

        let req = POST!("/account/v4/personal-access-token/{personal_access_token_id}/renew",)
            .body_json(&request)
            .context("Failed to create renew personal access token request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send renew personal access token request")?;

        let response: RenewPersonalAccessTokenResponseData = assert_response!(res);
        let new_token = response.personal_access_token.token;

        info!(
            "Personal access token renewed successfully: {}",
            personal_access_token_id
        );

        let pat_key_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&raw_pat_key);
        let env_var = format!("{}::{}", new_token, pat_key_b64);

        Ok(RenewPersonalAccessTokenResponse {
            token: new_token,
            env_var,
        })
    }
}
