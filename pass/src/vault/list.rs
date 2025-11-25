use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use futures::stream::{self, StreamExt};
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ShareContent, ShareId, ShareType, Vault, VaultData, VaultId};

const MAX_CONCURRENCY: usize = 20;

struct VaultListCacheType;

impl PassClient {
    pub async fn list_vaults(&self) -> Result<Vec<Vault>> {
        {
            let cached: Option<Vec<Vault>> = self.cache.get(VaultListCacheType).await;
            if let Some(cached_vaults) = cached {
                return Ok(cached_vaults);
            }
        }

        let shares = self.list_shares().await.context("Error listing shares")?;

        // Filter and collect vault shares with their original indices to preserve order
        let vault_shares: Vec<(usize, VaultId, ShareId, Option<ShareContent>)> = shares
            .into_iter()
            .enumerate()
            .filter_map(|(index, share)| {
                if let ShareType::Vault { vault_id } = share.share_type {
                    Some((index, vault_id, share.id, share.content))
                } else {
                    None
                }
            })
            .collect();

        if vault_shares.is_empty() {
            return Ok(vec![]);
        }

        // Execute futures concurrently with built-in parallelism limiting
        let results: Vec<(usize, Vault)> = stream::iter(vault_shares)
            .map(|(original_index, vault_id, share_id, share_content)| {
                let client = self.clone();
                async move {
                    match client
                        .open_vault_share_content(&share_id, share_content)
                        .await
                    {
                        Ok(content) => Some((
                            original_index,
                            Vault {
                                id: vault_id,
                                share_id,
                                content,
                            },
                        )),
                        Err(e) => {
                            error!("Error opening share {}: {}", share_id, e);
                            None
                        }
                    }
                }
            })
            .buffered(MAX_CONCURRENCY)
            .filter_map(|result| async { result })
            .collect()
            .await;

        // Sort results by original index to preserve order
        let mut results = results;
        results.sort_by_key(|(index, _)| *index);

        let vaults: Vec<Vault> = results.into_iter().map(|(_, vault)| vault).collect();

        self.cache.store(VaultListCacheType, vaults.clone()).await;
        Ok(vaults)
    }

    pub async fn open_vault_share_content(
        &self,
        share_id: &ShareId,
        share_content: Option<ShareContent>,
    ) -> Result<VaultData> {
        let content = share_content
            .ok_or_else(|| anyhow!("Share {share_id} is of type vault and should have content"))?;

        let share_key = self
            .get_opened_share_key_by_rotation(share_id, content.share_key_rotation)
            .await
            .context("Error getting opened share key")?;

        let decrypted = pass_domain::crypto::decrypt(
            &content.content,
            share_key.as_ref(),
            EncryptionTag::VaultContent,
        )
        .map_err(|e| anyhow::anyhow!("Error decrypting VaultContent: {:?}", e))?;

        let parsed_content = VaultData::deserialize(&decrypted)
            .map_err(|e| anyhow::anyhow!("Error parsing vault content {:?}", e))?;

        Ok(parsed_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use crate::share::list::{GetSharesResponse, ShareResponse};
    use muon::test::server::{HTTP, Server};
    use pass_domain::{PermissionFlag, TargetType, VaultColor, VaultDisplayPreferences, VaultIcon};

    #[muon::test(scheme(HTTP))]
    async fn test_list_vaults_empty(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse { shares: vec![] })
        });

        let recorder = server.new_recorder();
        let vaults = client
            .list_vaults()
            .await
            .expect("Should be able to list vaults");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        assert!(vaults.is_empty());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_vaults_with_content_filters_item_share(server: Arc<Server>) {
        const SHARE_1_ID: &str = "Share1ID";
        const SHARE_1_VAULT_ID: &str = "Vault1";
        const SHARE_1_VAULT_NAME: &str = "Vault name";
        const SHARE_1_VAULT_DESCRIPTION: &str = "A test vault";
        const SHARE_2_ID: &str = "Share2ID";
        const SHARE_2_VAULT_ID: &str = "Share2VaultID";
        const SHARE_2_ITEM_ID: &str = "Share2ItemID";

        let client = server.pass_client().await;
        setup_share_keys(&server, SHARE_1_ID);

        let vault_data = VaultData::new(
            SHARE_1_VAULT_NAME.to_string(),
            SHARE_1_VAULT_DESCRIPTION.to_string(),
            VaultDisplayPreferences {
                icon: VaultIcon::Icon3,
                color: VaultColor::Color4,
            },
        )
        .unwrap();

        let serialized_vault_data = vault_data.clone().serialize().unwrap();
        let encrypted_vault_data =
            encrypt_for_vault_key(&serialized_vault_data, EncryptionTag::VaultContent);
        let encoded_encrypted_vault_data = crate::utils::b64_encode(&encrypted_vault_data);
        let handled = server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse {
                shares: vec![
                    ShareResponse {
                        share_id: SHARE_1_ID.to_string(),
                        address_id: TEST_ADDRESS_ID.to_string(),
                        vault_id: SHARE_1_VAULT_ID.to_string(),
                        target_type: TargetType::Vault.value(),
                        target_id: SHARE_1_VAULT_ID.to_string(),
                        owner: true,
                        permission: PermissionFlag::Admin.value(),
                        share_role_id: "1".to_string(),
                        content: Some(encoded_encrypted_vault_data.to_string()),
                        content_key_rotation: Some(1),
                        content_format_version: Some(1),
                        expiration_time: None,
                        create_time: 12345678,
                        group_id: None,
                    },
                    ShareResponse {
                        share_id: SHARE_2_ID.to_string(),
                        address_id: TEST_ADDRESS_ID.to_string(),
                        vault_id: SHARE_2_VAULT_ID.to_string(),
                        target_type: TargetType::Item.value(),
                        target_id: SHARE_2_ITEM_ID.to_string(),
                        owner: false,
                        permission: PermissionFlag::Admin.value(),
                        share_role_id: "1".to_string(),
                        content: None,
                        content_key_rotation: None,
                        content_format_version: None,
                        expiration_time: None,
                        create_time: 12345678,
                        group_id: None,
                    },
                ],
            })
        });

        let recorder = server.new_recorder();
        let vaults = client
            .list_vaults()
            .await
            .expect("Should be able to list vaults");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(3, requests.len());

        // Only 1 vault should be returned, as the other share is of type item
        assert_eq!(1, vaults.len());

        let vault = &vaults[0];
        assert_eq!(SHARE_1_VAULT_ID, vault.id.value());
        assert_eq!(SHARE_1_ID, vault.share_id.value());
        assert_eq!(vault_data, vault.content);
    }
}
