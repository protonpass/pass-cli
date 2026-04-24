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

use crate::common::CodeResponse;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::POST;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ItemId, PersonalAccessTokenId, ShareId, ShareRole, TargetType, crypto};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct KeyRotationKeyPair {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct PersonalAccessTokenGrantAccessRequest {
    #[serde(rename = "ShareID")]
    share_id: String,
    #[serde(rename = "TargetID", skip_serializing_if = "Option::is_none")]
    target_id: Option<String>,
    #[serde(rename = "TargetType")]
    target_type: u8,
    #[serde(rename = "ShareRoleID")]
    share_role_id: String,
    #[serde(rename = "Keys")]
    keys: Vec<KeyRotationKeyPair>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn grant_personal_access_token_access(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
        share_id: &ShareId,
        item_id: Option<&ItemId>,
        role: &ShareRole,
    ) -> Result<()> {
        self.personal_access_token_operation_guard()?;
        info!(
            "Granting personal access token {} access to share {} (item: {:?})",
            personal_access_token_id, share_id, item_id
        );

        // Prepare the access grant request
        let request = self
            .prepare_grant_access_request(personal_access_token_id, share_id, item_id, role)
            .await?;

        // Send the request to the server
        self.send_grant_access_request(personal_access_token_id, request)
            .await?;

        info!("Personal access token {personal_access_token_id} granted access successfully",);

        Ok(())
    }

    async fn prepare_grant_access_request(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
        share_id: &ShareId,
        item_id: Option<&ItemId>,
        role: &ShareRole,
    ) -> Result<PersonalAccessTokenGrantAccessRequest> {
        let personal_access_token_key = self
            .get_personal_access_token_key(personal_access_token_id)
            .await
            .context("Failed to get personal access token key")?;

        let (target_type, target_id, keys) = match item_id {
            Some(item_id) => {
                self.prepare_item_access_keys(share_id, item_id, &personal_access_token_key)
                    .await?
            }
            None => {
                self.prepare_vault_access_keys(share_id, &personal_access_token_key)
                    .await?
            }
        };

        Ok(PersonalAccessTokenGrantAccessRequest {
            share_id: share_id.value().to_string(),
            target_id,
            target_type,
            share_role_id: role.value(),
            keys,
        })
    }

    async fn prepare_vault_access_keys(
        &self,
        share_id: &ShareId,
        personal_access_token_key: &[u8],
    ) -> Result<(u8, Option<String>, Vec<KeyRotationKeyPair>)> {
        let share_keys = self
            .get_all_opened_share_keys(share_id, true)
            .await
            .context("Error getting opened share keys")?;

        let encrypted_keys: Vec<KeyRotationKeyPair> = share_keys
            .into_iter()
            .map(|k| {
                let encrypted_key =
                    crypto::encrypt(k.key(), personal_access_token_key, EncryptionTag::ShareKey)
                        .map_err(|e| {
                            error!("Error encrypting vault key: {:?}", e);
                            anyhow!("Error encrypting vault key")
                        })?;

                Ok(KeyRotationKeyPair {
                    key: crate::utils::b64_encode(encrypted_key),
                    key_rotation: k.key_rotation,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((TargetType::Vault.value(), None, encrypted_keys))
    }

    async fn prepare_item_access_keys(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        personal_access_token_key: &[u8],
    ) -> Result<(u8, Option<String>, Vec<KeyRotationKeyPair>)> {
        let item_keys = self
            .get_item_keys(share_id, item_id)
            .await
            .context("Error getting item keys")?;

        let opened_item_keys = self
            .open_item_keys(share_id, item_keys)
            .await
            .context("Error opening item keys")?;

        let encrypted_keys: Vec<KeyRotationKeyPair> = opened_item_keys
            .into_iter()
            .map(|k| {
                let encrypted_key = crypto::encrypt(
                    k.key.as_ref(),
                    personal_access_token_key,
                    EncryptionTag::ShareKey,
                )
                .map_err(|e| {
                    error!("Error encrypting item key: {:?}", e);
                    anyhow!("Error encrypting item key")
                })?;

                Ok(KeyRotationKeyPair {
                    key: crate::utils::b64_encode(encrypted_key),
                    key_rotation: k.key_rotation,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            TargetType::Item.value(),
            Some(item_id.value().to_string()),
            encrypted_keys,
        ))
    }

    async fn send_grant_access_request(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
        request: PersonalAccessTokenGrantAccessRequest,
    ) -> Result<()> {
        let req = POST!("/pass/v1/personal-access-token/{personal_access_token_id}/access",)
            .body_json(&request)
            .context("Failed to create grant access request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send grant access request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }

    pub(crate) async fn get_personal_access_token_key(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
    ) -> Result<Vec<u8>> {
        self.personal_access_token_operation_guard()?;
        let personal_access_tokens = self
            .list_personal_access_tokens()
            .await
            .context("Failed to list personal access tokens")?;

        let personal_access_token = personal_access_tokens
            .iter()
            .find(|pat| personal_access_token_id.eq(&pat.pat_id))
            .ok_or_else(|| {
                anyhow!("Personal access token not found: {personal_access_token_id}",)
            })?;

        personal_access_token
            .pat_key
            .clone()
            .ok_or_else(|| anyhow!("Personal access token key not available"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::personal_access_token::list::{
        ListPersonalAccessTokensResponse, PersonalAccessTokenData, PersonalAccessTokensWrapper,
    };
    use crate::test_tools::*;
    use pass_domain::PlainText;

    #[muon_test::test]
    async fn test_grant_vault_access(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const PERSONAL_ACCESS_TOKEN_ID: &str = "test_sa_id";
        const SHARE_ID: &str = "test_share_id";
        const VAULT_ID: &str = "test_vault_id";
        const GRANT_PATH: &str = "/pass/v1/personal-access-token/test_sa_id/access";
        const SHARE_KEY_PATH: &str = "/pass/v1/share/test_share_id/key";

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

        let name = "TestPersonalAccessToken".to_string();
        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_personal_access_token_key);

        let list_handled = api.handler("/account/v4/personal-access-token", move |_| {
            success(ListPersonalAccessTokensResponse {
                personal_access_tokens: PersonalAccessTokensWrapper {
                    personal_access_tokens: vec![PersonalAccessTokenData {
                        personal_access_token_id: PERSONAL_ACCESS_TOKEN_ID.to_string(),
                        name: name.clone(),
                        personal_access_token_key: encrypted_key_b64.clone(),
                        expire_time: None,
                        create_time: 1704067200,
                        modify_time: 1704067200,
                        flags: None,
                    }],
                    total: 1,
                    last_token: None,
                },
            })
        });

        // Mock share keys endpoint
        let share_key_raw = crypto::generate_encryption_key();
        let encrypted_share_key = client.encrypt_for_user_key(share_key_raw.clone()).await;
        let encrypted_share_key_b64 = crate::utils::b64_encode(encrypted_share_key.clone());

        let share_key_handled = api.handler(SHARE_KEY_PATH, move |_| {
            use crate::share::keys::{GetShareKeysResponse, ShareKeyList, ShareKeyResponse};
            success(GetShareKeysResponse {
                keys: ShareKeyList {
                    keys: vec![ShareKeyResponse {
                        key_rotation: 1,
                        key: encrypted_share_key_b64.clone(),
                        create_time: 1704067200,
                    }],
                    total: 1,
                },
            })
        });

        // Mock shares list endpoint
        let share_handled = api.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            use crate::share::list::{GetSharesResponse, ShareResponse};
            success(GetSharesResponse {
                shares: vec![ShareResponse {
                    share_id: SHARE_ID.to_string(),
                    address_id: TEST_ADDRESS_ID.to_string(),
                    vault_id: VAULT_ID.to_string(),
                    target_type: TargetType::Vault.value(),
                    target_id: VAULT_ID.to_string(),
                    owner: true,
                    permission: 0,
                    share_role_id: ShareRole::Owner.value(),
                    content: None,
                    content_key_rotation: None,
                    content_format_version: None,
                    expiration_time: None,
                    create_time: 1704067200,
                    group_id: None,
                }],
            })
        });

        let grant_handled = api.handler(GRANT_PATH, |_| success_code());

        let recorder = api.new_recorder();

        client
            .grant_personal_access_token_access(
                &PersonalAccessTokenId::new(PERSONAL_ACCESS_TOKEN_ID.to_string()),
                &ShareId::new(SHARE_ID.to_string()),
                None,
                &ShareRole::Viewer,
            )
            .await
            .expect("Should be able to grant vault access");

        assert_hit!(list_handled);
        assert_hit!(share_key_handled);
        assert_hit!(share_handled);
        assert_hit!(grant_handled);

        let req: PersonalAccessTokenGrantAccessRequest = last_request!(recorder);

        assert_eq!(SHARE_ID, req.share_id);
        assert_eq!(None, req.target_id);
        assert_eq!(TargetType::Vault.value(), req.target_type);
        assert_eq!(ShareRole::Viewer.value(), req.share_role_id);
        assert!(!req.keys.is_empty());
    }
}
