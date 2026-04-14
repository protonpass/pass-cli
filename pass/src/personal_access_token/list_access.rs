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

use crate::pagination::SincePagination;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{ItemId, PersonalAccessTokenId, ShareId, ShareRole, TargetType};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PersonalAccessTokenAccess {
    Vault {
        share_id: ShareId,
        role: ShareRole,
        #[serde(skip_serializing_if = "Option::is_none")]
        expire_time: Option<i64>,
        vault_name: String,
    },
    Item {
        share_id: ShareId,
        role: ShareRole,
        #[serde(skip_serializing_if = "Option::is_none")]
        expire_time: Option<i64>,
        item_title: String,
        item_id: ItemId,
    },
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ListPersonalAccessTokenAccessResponse {
    #[serde(rename = "Shares")]
    shares: Vec<PersonalAccessTokenShare>,
    #[serde(rename = "LastToken")]
    last_token: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PersonalAccessTokenShare {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "ParentShareID")]
    pub parent_share_id: String,
    #[serde(rename = "TargetType")]
    pub target_type: u8,
    #[serde(rename = "TargetID")]
    pub target_id: Option<String>,
    #[serde(rename = "ShareRoleID")]
    pub share_role_id: String,
    #[serde(rename = "ExpireTime")]
    pub expire_time: Option<i64>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn list_personal_access_token_access(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
    ) -> Result<Vec<PersonalAccessTokenAccess>> {
        self.personal_access_token_operation_guard()?;
        let access_list = self
            .fetch_personal_access_token_access(personal_access_token_id)
            .await
            .context("Error fetching personal access token access")?;

        let mut result = Vec::with_capacity(access_list.len());
        for access in access_list {
            let resolved = self
                .resolve_personal_access_token_access(access)
                .await
                .context("Error resolving personal access token access")?;
            result.push(resolved);
        }

        Ok(result)
    }

    async fn resolve_personal_access_token_access(
        &self,
        response: PersonalAccessTokenShare,
    ) -> Result<PersonalAccessTokenAccess> {
        let parent_share_id = ShareId::new(response.parent_share_id);
        let share_id = ShareId::new(response.share_id);
        let role = ShareRole::from_value(&response.share_role_id, false, 0);
        let target_type =
            TargetType::from_value(response.target_type).context("Invalid target type")?;

        match target_type {
            TargetType::Vault => {
                let share = self
                    .get_share(&parent_share_id)
                    .await
                    .context("Failed to get share")?;

                let vault_data = self
                    .open_vault_share_content(&parent_share_id, share.content)
                    .await
                    .context("Failed to decrypt vault content")?;

                Ok(PersonalAccessTokenAccess::Vault {
                    share_id,
                    role,
                    expire_time: response.expire_time,
                    vault_name: vault_data.name,
                })
            }
            TargetType::Item => {
                let item_id_str = response
                    .target_id
                    .ok_or_else(|| anyhow!("Item ID not provided for item target"))?;
                let item_id = ItemId::new(item_id_str.clone());

                let items = self
                    .list_items(&parent_share_id)
                    .await
                    .context("Failed to list items")?;

                let item = items
                    .iter()
                    .find(|i| i.id.value() == item_id.value())
                    .ok_or_else(|| anyhow!("Item not found: {}", item_id_str))?;

                Ok(PersonalAccessTokenAccess::Item {
                    share_id,
                    role,
                    item_id,
                    expire_time: response.expire_time,
                    item_title: item.content.title.clone(),
                })
            }
        }
    }

    async fn fetch_personal_access_token_access(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
    ) -> Result<Vec<PersonalAccessTokenShare>> {
        let mut access_list = vec![];
        let mut pagination = SincePagination::default();

        loop {
            let mut req = GET!(
                "/pass/v1/personal-access-token/{}/access",
                personal_access_token_id
            );
            if let Some(ref since) = pagination.since {
                req = req.query(("Since", since));
            }

            let res = self
                .send(req)
                .await
                .context("Error fetching personal access token access")?;
            let response: ListPersonalAccessTokenAccessResponse = assert_response!(res);

            let should_break = response.shares.len() < pagination.page_size;
            access_list.extend(response.shares);

            if should_break {
                break;
            }

            pagination = match pagination.next(response.last_token) {
                Some(p) => p,
                None => break,
            };
        }

        Ok(access_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    #[muon_test::test]
    async fn test_list_personal_access_token_access_vault(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        use crate::share::keys::{GetShareKeysResponse, ShareKeyList, ShareKeyResponse};
        use crate::share::list::{GetSharesResponse, ShareResponse};
        use pass_domain::{VaultData, VaultDisplayPreferences, crypto};

        const PERSONAL_ACCESS_TOKEN_ID: &str = "test_sa_id";
        const SHARE_ID: &str = "share_1";
        const PARENT_SHARE_ID: &str = "parent_share_1";
        const VAULT_ID: &str = "vault_1";
        const VAULT_NAME: &str = "Test Vault";
        const LIST_ACCESS_PATH: &str = "/pass/v1/personal-access-token/test_sa_id/access";
        const SHARE_KEY_PATH: &str = "/pass/v1/share/parent_share_1/key";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        // Prepare vault content
        let vault_data = VaultData {
            name: VAULT_NAME.to_string(),
            description: "Test description".to_string(),
            display_preferences: VaultDisplayPreferences::default(),
        };
        let vault_content_bytes = vault_data
            .serialize()
            .expect("Failed to serialize vault data");
        let share_key_raw = crypto::generate_encryption_key();
        let encrypted_share_key = client.encrypt_for_user_key(share_key_raw.clone()).await;
        let encrypted_vault_content = crypto::encrypt(
            &vault_content_bytes,
            &share_key_raw,
            pass_domain::crypto::EncryptionTag::VaultContent,
        )
        .expect("Failed to encrypt vault content");
        let encrypted_vault_content_b64 = crate::utils::b64_encode(encrypted_vault_content);
        let encrypted_share_key_b64 = crate::utils::b64_encode(encrypted_share_key);

        // Mock share key endpoint
        api.handler(SHARE_KEY_PATH, move |_| {
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

        // Mock shares endpoint
        api.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse {
                shares: vec![ShareResponse {
                    share_id: PARENT_SHARE_ID.to_string(),
                    address_id: TEST_ADDRESS_ID.to_string(),
                    vault_id: VAULT_ID.to_string(),
                    target_type: TargetType::Vault.value(),
                    target_id: VAULT_ID.to_string(),
                    owner: true,
                    permission: 0,
                    share_role_id: ShareRole::Owner.value(),
                    content: Some(encrypted_vault_content_b64.clone()),
                    content_key_rotation: Some(1),
                    content_format_version: Some(1),
                    expiration_time: None,
                    create_time: 1704067200,
                    group_id: None,
                }],
            })
        });

        let list_access_handled = api.handler(LIST_ACCESS_PATH, move |_| {
            success(ListPersonalAccessTokenAccessResponse {
                shares: vec![PersonalAccessTokenShare {
                    share_id: SHARE_ID.to_string(),
                    target_type: TargetType::Vault.value(),
                    target_id: None,
                    share_role_id: ShareRole::Viewer.value(),
                    expire_time: None,
                    parent_share_id: PARENT_SHARE_ID.to_string(),
                }],
                last_token: None,
            })
        });

        let access_list = client
            .list_personal_access_token_access(&PersonalAccessTokenId::new(
                PERSONAL_ACCESS_TOKEN_ID.to_string(),
            ))
            .await
            .expect("Should be able to list personal access token access");

        assert_hit!(list_access_handled);
        assert_eq!(1, access_list.len());

        match &access_list[0] {
            PersonalAccessTokenAccess::Vault {
                share_id,
                role,
                expire_time,
                vault_name,
            } => {
                assert_eq!(SHARE_ID, share_id.value());
                assert_eq!(ShareRole::Viewer, *role);
                assert_eq!(None, *expire_time);
                assert_eq!(VAULT_NAME, vault_name);
            }
            _ => panic!("Expected Vault variant"),
        }
    }
}
