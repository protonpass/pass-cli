use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ShareContent, ShareId, ShareType, Vault, VaultData};

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
        let mut vaults = vec![];

        for share in shares {
            if let ShareType::Vault { vault_id } = share.share_type {
                let parsed = self
                    .open_vault_share_content(&share.id, share.content)
                    .await;
                match parsed {
                    Ok(content) => {
                        vaults.push(Vault {
                            id: vault_id.clone(),
                            share_id: share.id,
                            content,
                        });
                    }
                    Err(e) => {
                        error!("Error opening share {}: {}", share.id, e);
                    }
                }
            }
        }

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
