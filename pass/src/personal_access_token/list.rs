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
use muon::GET;
use pass_domain::PersonalAccessTokenId;

const PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PersonalAccessTokenData {
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
    #[serde(rename = "Flags")]
    pub flags: Option<u64>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct PersonalAccessTokensWrapper {
    #[serde(rename = "PersonalAccessTokens")]
    pub(crate) personal_access_tokens: Vec<PersonalAccessTokenData>,
    #[serde(rename = "Total")]
    #[allow(dead_code)]
    pub(crate) total: i64,
    #[serde(rename = "LastToken")]
    pub(crate) last_token: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct ListPersonalAccessTokensResponse {
    #[serde(rename = "PersonalAccessTokens")]
    pub(crate) personal_access_tokens: PersonalAccessTokensWrapper,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PersonalAccessToken {
    pub pat_id: PersonalAccessTokenId,
    pub name: String,
    pub expire_time: Option<i64>,
    #[serde(skip)]
    pub flags: Option<u64>,
    #[serde(skip)]
    pub(crate) pat_key: Option<Vec<u8>>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn list_personal_access_tokens(&self) -> Result<Vec<PersonalAccessToken>> {
        self.personal_access_token_operation_guard()?;
        info!("Fetching personal access tokens");

        let mut all_personal_access_tokens = Vec::new();
        let mut last_token: Option<String> = None;

        loop {
            let mut req = GET!("/account/v4/personal-access-token")
                .query(("PageSize", format!("{}", PAGE_SIZE)))
                .query(("Product", "pass".to_string()));

            if let Some(token) = &last_token {
                req = req.query(("Since", token.clone()));
            }

            let res = self
                .send(req)
                .await
                .context("Error sending list personal access tokens request")?;

            let response: ListPersonalAccessTokensResponse = assert_response!(res);

            let wrapper = response.personal_access_tokens;

            for pat_data in wrapper.personal_access_tokens {
                match self.open_personal_access_token(&pat_data).await {
                    Ok(pat) => all_personal_access_tokens.push(pat),
                    Err(e) => {
                        warn!(
                            "Error opening personal access token {}: {}. Skipping.",
                            pat_data.personal_access_token_id, e
                        );
                    }
                }
            }

            match wrapper.last_token {
                Some(token) if !token.is_empty() => {
                    last_token = Some(token);
                }
                _ => break,
            }
        }

        info!(
            "Successfully fetched {} personal access tokens",
            all_personal_access_tokens.len()
        );

        Ok(all_personal_access_tokens)
    }

    async fn open_personal_access_token(
        &self,
        pat_data: &PersonalAccessTokenData,
    ) -> Result<PersonalAccessToken> {
        let encrypted_personal_access_token_key =
            crate::utils::b64_decode(&pat_data.personal_access_token_key)
                .context("Error decoding personal access token key")?;

        let user_key = self
            .get_primary_user_key()
            .await
            .context("Error getting primary user key")?;
        let (private, public) = user_key.into_keys();
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let decrypted_personal_access_token_key = pgp_crypto
            .decrypt_and_verify(
                encrypted_personal_access_token_key,
                vec![private],
                vec![public],
                None,
            )
            .await
            .context("Error decrypting and verifying personal access token key")?;
        Ok(PersonalAccessToken {
            pat_id: PersonalAccessTokenId::new(pat_data.personal_access_token_id.clone()),
            name: pat_data.name.to_string(),
            expire_time: pat_data.expire_time,
            flags: pat_data.flags,
            pat_key: Some(decrypted_personal_access_token_key),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use pass_domain::PlainText;
    use pass_domain::crypto;

    #[muon_test::test]
    async fn test_list_personal_access_tokens_empty(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        let handled = api.handler("/account/v4/personal-access-token", |_| {
            success(ListPersonalAccessTokensResponse {
                personal_access_tokens: PersonalAccessTokensWrapper {
                    personal_access_tokens: vec![],
                    total: 0,
                    last_token: None,
                },
            })
        });

        let result = client
            .list_personal_access_tokens()
            .await
            .expect("Should be able to list personal access tokens");

        assert_eq!(0, result.len());
        assert_hit!(handled);
    }

    #[muon_test::test]
    async fn test_list_personal_access_tokens_single(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const PERSONAL_ACCESS_TOKEN_NAME: &str = "MyTestPersonalAccessToken";
        const PERSONAL_ACCESS_TOKEN_ID: &str = "test_id_123";
        const CREATE_TIME: i64 = 1704067200;
        const MODIFY_TIME: i64 = 1704067200;

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        let personal_access_token_key = crypto::generate_encryption_key();

        let user_key = client.get_primary_user_key().await.unwrap();
        let (private, public) = user_key.into_keys();
        let pgp_crypto = client.client_features.get_pgp_crypto().await;

        let encrypted_personal_access_token_key = pgp_crypto
            .encrypt_and_sign(
                PlainText::new(personal_access_token_key.clone()),
                public,
                private,
                None,
            )
            .await
            .expect("Error encrypting personal access token key");

        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_personal_access_token_key);

        let handled = api.handler("/account/v4/personal-access-token", move |_| {
            success(ListPersonalAccessTokensResponse {
                personal_access_tokens: PersonalAccessTokensWrapper {
                    personal_access_tokens: vec![PersonalAccessTokenData {
                        personal_access_token_id: PERSONAL_ACCESS_TOKEN_ID.to_string(),
                        name: PERSONAL_ACCESS_TOKEN_NAME.to_string(),
                        personal_access_token_key: encrypted_key_b64.clone(),
                        expire_time: None,
                        create_time: CREATE_TIME,
                        modify_time: MODIFY_TIME,
                        flags: None,
                    }],
                    total: 1,
                    last_token: None,
                },
            })
        });

        let result = client
            .list_personal_access_tokens()
            .await
            .expect("Should be able to list personal access tokens");

        assert_eq!(1, result.len());
        assert_eq!(PERSONAL_ACCESS_TOKEN_ID, result[0].pat_id.value());
        assert_eq!(PERSONAL_ACCESS_TOKEN_NAME, result[0].name);
        assert_eq!(None, result[0].expire_time);

        assert_hit!(handled);
    }

    #[muon_test::test]
    async fn test_list_personal_access_tokens_with_expiration(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const PERSONAL_ACCESS_TOKEN_NAME: &str = "ExpiringAccount";
        const PERSONAL_ACCESS_TOKEN_ID: &str = "expiring_id";
        const EXPIRATION_TIME: i64 = 1735689600;

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        let personal_access_token_key = crypto::generate_encryption_key();

        let user_key = client.get_primary_user_key().await.unwrap();
        let (private, public) = user_key.into_keys();
        let pgp_crypto = client.client_features.get_pgp_crypto().await;

        let encrypted_personal_access_token_key = pgp_crypto
            .encrypt_and_sign(
                PlainText::new(personal_access_token_key.clone()),
                public,
                private,
                None,
            )
            .await
            .expect("Error encrypting personal access token key");

        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_personal_access_token_key);

        let handled = api.handler("/account/v4/personal-access-token", move |_| {
            success(ListPersonalAccessTokensResponse {
                personal_access_tokens: PersonalAccessTokensWrapper {
                    personal_access_tokens: vec![PersonalAccessTokenData {
                        personal_access_token_id: PERSONAL_ACCESS_TOKEN_ID.to_string(),
                        name: PERSONAL_ACCESS_TOKEN_NAME.to_string(),
                        personal_access_token_key: encrypted_key_b64.clone(),
                        expire_time: Some(EXPIRATION_TIME),
                        create_time: 1704067200,
                        modify_time: 1704067200,
                        flags: None,
                    }],
                    total: 1,
                    last_token: None,
                },
            })
        });

        let result = client
            .list_personal_access_tokens()
            .await
            .expect("Should be able to list personal access tokens");

        assert_eq!(1, result.len());
        assert_eq!(Some(EXPIRATION_TIME), result[0].expire_time);

        assert_hit!(handled);
    }
}
