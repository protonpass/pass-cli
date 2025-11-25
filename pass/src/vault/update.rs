use crate::PassClient;
use crate::permission::PermissionAction;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_domain::{ShareId, VaultData, crypto};

pub struct UpdateVaultArgs {
    name: String,
}

impl UpdateVaultArgs {
    pub fn new(name: String) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(anyhow!("Empty vault name"));
        }

        Ok(Self { name })
    }
}
#[derive(Clone, Debug, serde::Serialize)]
struct UpdateVaultRequest {
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: u32,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
}

impl PassClient {
    pub async fn update_vault(&self, share_id: &ShareId, args: UpdateVaultArgs) -> Result<()> {
        self.action_guard(PermissionAction::UpdateVault {
            share_id: share_id.clone(),
        })
        .await?;
        let req = self
            .update_vault_request(share_id, args)
            .await
            .context("Failed to create Update Vault request")?;
        let req = PUT!("/pass/v1/vault/{}", share_id)
            .body_json(&req)
            .context("Failed to update Vault request")?;
        let res = self
            .send(req)
            .await
            .context("Failed to send create Vault request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Failed to create Vault request: {}", res.status()));
        }

        self.clear_shares_cache().await;

        Ok(())
    }

    async fn update_vault_request(
        &self,
        share_id: &ShareId,
        args: UpdateVaultArgs,
    ) -> Result<UpdateVaultRequest> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        if !share.is_vault_share() {
            return Err(anyhow!("Only Vault shares can be updated"));
        }

        let share_content = match share.content {
            Some(ref content) => content,
            None => return Err(anyhow::anyhow!("Share should have vault content")),
        };

        let opened_share_key = self
            .get_opened_share_key_by_rotation(share_id, share_content.share_key_rotation)
            .await
            .context("Failed to get opened share key")?;

        let decrypted_content = crypto::decrypt(
            &share_content.content,
            opened_share_key.as_ref(),
            crypto::EncryptionTag::VaultContent,
        )
        .map_err(|e| {
            error!(
                "Error decrypting share content for share {}: {:?}",
                share_id, e
            );
            anyhow::anyhow!("Error decrypting share content for vault")
        })?;

        let mut parsed =
            VaultData::deserialize(&decrypted_content).context("Error parsing vault content")?;
        parsed.name = args.name;

        let serialized = parsed
            .serialize()
            .context("Error serializing vault content")?;

        // Get latest key rotation from API to ensure we use the most recent
        let share_keys = self
            .get_share_keys(share_id)
            .await
            .context("Error getting share keys")?;
        let encryption_key = match share_keys.latest() {
            Some(k) => k.clone(),
            None => return Err(anyhow::anyhow!("Error getting latest vault encryption key")),
        };

        let encryption_key_rotation = encryption_key.key_rotation;
        let opened_encryption_key = self
            .get_opened_share_key_by_rotation(share_id, encryption_key_rotation)
            .await
            .context("Failed to open encryption key")?;

        let encrypted = crypto::encrypt(
            &serialized,
            opened_encryption_key.as_ref(),
            crypto::EncryptionTag::VaultContent,
        )
        .map_err(|e| {
            error!("Error encrypting vault: {}", e);
            anyhow::anyhow!("Error encrypting vault")
        })?;

        Ok(UpdateVaultRequest {
            content: crate::utils::b64_encode(encrypted),
            content_format_version: crate::constants::VAULT_CONTENT_CONTENT_FORMAT_VERSION,
            key_rotation: encryption_key_rotation,
        })
    }
}
