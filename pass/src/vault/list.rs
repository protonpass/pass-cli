use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use futures::stream::{self, StreamExt};
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ShareContent, ShareId, ShareType, Vault, VaultData, VaultId};

const MAX_CONCURRENCY: usize = 10;

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

        let keys = self
            .get_share_keys(share_id)
            .await
            .context("Error retrieving share keys")?;

        let key = match keys.find_by_rotation(content.share_key_rotation) {
            Some(key) => key.clone(),
            None => {
                return Err(anyhow!(
                    "Could not find ShareKey for Share {} with rotation {}",
                    share_id,
                    content.share_key_rotation
                ));
            }
        };

        let share_key = self
            .open_share_key(key)
            .await
            .context("Error opening share key")?;
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
