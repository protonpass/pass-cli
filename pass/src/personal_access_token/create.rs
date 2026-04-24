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

use super::PersonalAccessTokenFlags;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use base64::Engine;
use muon::POST;
use pass_domain::{PersonalAccessTokenId, PlainText, crypto};

#[derive(Debug)]
pub struct CreatePersonalAccessTokenArgs {
    name: String,
    expiration_time: i64,
    pass_agent: bool,
}

impl CreatePersonalAccessTokenArgs {
    pub fn new(name: String, expiration_time: i64) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(anyhow!("Empty personal access token name"));
        }

        Ok(Self {
            name,
            expiration_time,
            pass_agent: false,
        })
    }

    pub fn with_pass_agent_flag(mut self) -> Self {
        self.pass_agent = true;
        self
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreatePersonalAccessTokenRequest {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PersonalAccessTokenKey")]
    pub personal_access_token_key: String,
    #[serde(rename = "ExpireTime")]
    pub expire_time: i64,
    #[serde(rename = "Products")]
    pub products: Vec<String>,
    #[serde(rename = "Flags")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<PersonalAccessTokenFlags>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreatePersonalAccessTokenResponseData {
    #[serde(rename = "PersonalAccessToken")]
    pub personal_access_token: PersonalAccessTokenData,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct PersonalAccessTokenData {
    #[serde(rename = "PersonalAccessTokenID")]
    pub personal_access_token_id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PersonalAccessTokenKey")]
    pub personal_access_token_key: String,
    #[serde(rename = "ExpireTime")]
    pub expire_time: Option<i64>,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "ModifyTime")]
    pub modify_time: i64,
    #[serde(rename = "Token")]
    pub token: String,
}

pub struct CreatePersonalAccessTokenResponse {
    pub personal_access_token_id: PersonalAccessTokenId,
    pub name: String,
    pub personal_access_token_key: String,
    pub expire_time: Option<i64>,
    pub create_time: i64,
    pub modify_time: i64,
    pub token: String,
    pub raw_personal_access_token_key: Vec<u8>,
    pub env_var: String,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn create_personal_access_token(
        &self,
        args: CreatePersonalAccessTokenArgs,
    ) -> Result<CreatePersonalAccessTokenResponse> {
        self.personal_access_token_operation_guard()?;
        info!("Creating personal access token: {}", args.name);

        let (req, raw_personal_access_token_key) = self
            .create_personal_access_token_request(args)
            .await
            .context("Failed to create personal access token request")?;

        let req = POST!("/account/v4/personal-access-token")
            .body_json(&req)
            .context("Failed to create personal access token request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send create personal access token request")?;

        let response: CreatePersonalAccessTokenResponseData = assert_response!(res);

        let pat = response.personal_access_token;
        info!(
            "Personal access token created successfully: ID={}",
            pat.personal_access_token_id
        );

        // Encode with URL_SAFE_NO_PAD to make sure we don't break any shell / URL
        let personal_access_token_key_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&raw_personal_access_token_key);

        let env_var = format!("{}::{}", pat.token, personal_access_token_key_b64);
        Ok(CreatePersonalAccessTokenResponse {
            personal_access_token_id: PersonalAccessTokenId::new(pat.personal_access_token_id),
            name: pat.name,
            personal_access_token_key: pat.personal_access_token_key,
            expire_time: pat.expire_time,
            create_time: pat.create_time,
            modify_time: pat.modify_time,
            token: pat.token,
            env_var,
            raw_personal_access_token_key,
        })
    }

    async fn create_personal_access_token_request(
        &self,
        args: CreatePersonalAccessTokenArgs,
    ) -> Result<(CreatePersonalAccessTokenRequest, Vec<u8>)> {
        let personal_access_token_key = crypto::generate_encryption_key();

        let user_key = self
            .get_primary_user_key()
            .await
            .context("Error getting primary user key")?;
        let (private, public) = user_key.into_keys();
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let encrypted_personal_access_token_key = pgp_crypto
            .encrypt_and_sign(
                PlainText::new(personal_access_token_key.clone()),
                public,
                private,
                None,
            )
            .await
            .context("Error encrypting and signing personal access token key")?;

        let flags = if args.pass_agent {
            Some(PersonalAccessTokenFlags::for_agent())
        } else {
            None
        };

        Ok((
            CreatePersonalAccessTokenRequest {
                name: args.name,
                personal_access_token_key: crate::utils::b64_encode(
                    encrypted_personal_access_token_key,
                ),
                expire_time: args.expiration_time,
                products: vec!["pass".to_string()],
                flags,
            },
            personal_access_token_key,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    #[muon_test::test]
    async fn test_create_personal_access_token(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const PERSONAL_ACCESS_TOKEN_NAME: &str = "MyTestPersonalAccessToken";
        const PERSONAL_ACCESS_TOKEN_ID: &str = "MyPersonalAccessTokenID";
        const TOKEN: &str = "pst_test_token_123";
        const CREATE_TIME: i64 = 1704067200;
        const MODIFY_TIME: i64 = 1704067200;
        const EXPIRATION_TIME: i64 = 1735689600;

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        let handled = api.handler("/account/v4/personal-access-token", |_| {
            success(CreatePersonalAccessTokenResponseData {
                personal_access_token: PersonalAccessTokenData {
                    personal_access_token_id: PERSONAL_ACCESS_TOKEN_ID.to_string(),
                    name: "encrypted_name".to_string(),
                    personal_access_token_key: "encrypted_key".to_string(),
                    expire_time: Some(EXPIRATION_TIME),
                    create_time: CREATE_TIME,
                    modify_time: MODIFY_TIME,
                    token: TOKEN.to_string(),
                },
            })
        });

        let recorder = api.new_recorder();
        let response = client
            .create_personal_access_token(
                CreatePersonalAccessTokenArgs::new(
                    PERSONAL_ACCESS_TOKEN_NAME.to_string(),
                    EXPIRATION_TIME,
                )
                .unwrap(),
            )
            .await
            .expect("Should be able to create the personal access token");

        assert_eq!(
            PERSONAL_ACCESS_TOKEN_ID,
            response.personal_access_token_id.value()
        );
        assert_eq!(TOKEN, response.token);
        assert_eq!(32, response.raw_personal_access_token_key.len());

        assert_hit!(handled);

        let req: CreatePersonalAccessTokenRequest = last_request!(recorder);

        let user_key = client.get_primary_user_key().await.unwrap();
        let (private, public) = user_key.into_keys();

        let encrypted_personal_access_token_key =
            crate::utils::b64_decode(&req.personal_access_token_key).unwrap();
        let pgp_crypto = client.client_features.get_pgp_crypto().await;
        let decrypted_personal_access_token_key = pgp_crypto
            .decrypt_and_verify(
                encrypted_personal_access_token_key,
                vec![private],
                vec![public],
                None,
            )
            .await
            .expect("Error decrypting and verifying personal access token key");
        assert_eq!(32, decrypted_personal_access_token_key.len());
        assert_eq!(
            response.raw_personal_access_token_key,
            decrypted_personal_access_token_key
        );

        assert_eq!(PERSONAL_ACCESS_TOKEN_NAME, req.name);
        assert_eq!(EXPIRATION_TIME, req.expire_time);
    }

    #[test]
    fn test_empty_name_validation() {
        let result = CreatePersonalAccessTokenArgs::new("".to_string(), 1735689600);
        assert!(result.is_err());
        assert_eq!(
            "Empty personal access token name",
            result.unwrap_err().to_string()
        );

        let result = CreatePersonalAccessTokenArgs::new("   ".to_string(), 1735689600);
        assert!(result.is_err());
    }
}
