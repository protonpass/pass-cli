use crate::PassClient;
use crate::permission::PermissionAction;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::POST;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{PlainText, ShareId, VaultData, VaultDisplayPreferences, VaultId, crypto};

pub struct CreateVaultArgs {
    name: String,
}

impl CreateVaultArgs {
    pub fn new(name: String) -> Result<CreateVaultArgs> {
        if name.trim().is_empty() {
            return Err(anyhow!("Empty vault name"));
        }

        Ok(CreateVaultArgs { name })
    }
}
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreateVaultRequest {
    #[serde(rename = "AddressID")]
    pub address_id: String,
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: u32,
    #[serde(rename = "EncryptedVaultKey")]
    pub encrypted_vault_key: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreateVaultResponse {
    #[serde(rename = "Share")]
    pub share: CreateVaultResponseContent,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreateVaultResponseContent {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "VaultID")]
    pub vault_id: String,
}

impl PassClient {
    pub async fn create_vault(&self, args: CreateVaultArgs) -> Result<(ShareId, VaultId)> {
        self.action_guard(PermissionAction::CreateVault).await?;
        let req = self
            .create_vault_request(args)
            .await
            .context("Failed to create Vault request")?;
        let req = POST!("/pass/v1/vault")
            .body_json(&req)
            .context("Failed to create Vault request")?;
        let res = self
            .client
            .send(req)
            .await
            .context("Failed to send create Vault request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Failed to create Vault request: {}", res.status()));
        }

        let response: CreateVaultResponse =
            res.body_json().context("Failed to parse vault response")?;

        self.clear_shares_cache().await;
        Ok((
            ShareId::new(response.share.share_id),
            VaultId::new(response.share.vault_id),
        ))
    }

    async fn create_vault_request(&self, args: CreateVaultArgs) -> Result<CreateVaultRequest> {
        let addresses = self
            .get_addresses()
            .await
            .context("error getting addresses")?;
        let address = match addresses.first() {
            Some(address) => address,
            None => return Err(anyhow::anyhow!("empty address list")),
        };

        let content = VaultData::new(
            args.name,
            "Vault created from Pass CLI".to_string(),
            VaultDisplayPreferences::default(),
        )
        .context("Error in vault creation arguments")?
        .serialize()
        .context("Error serializing vault content")?;

        let vault_key = crypto::generate_encryption_key();
        let encrypted_content = crypto::encrypt(&content, &vault_key, EncryptionTag::VaultContent)
            .map_err(|e| {
                error!("Error encrypting content: {:?}", e);
                anyhow!("Error encrypting content")
            })?;

        let user_key = self
            .get_primary_user_key()
            .await
            .context("Error getting primary user key")?;
        let (private, public) = user_key.into_keys();
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let encrypted_vault_key = pgp_crypto
            .encrypt_and_sign(PlainText::new(vault_key), public, private, None)
            .await
            .context("Error encrypting and signing vault key")?;

        Ok(CreateVaultRequest {
            address_id: address.id.to_string(),
            content: crate::utils::b64_encode(encrypted_content),
            content_format_version: crate::constants::VAULT_CONTENT_CONTENT_FORMAT_VERSION,
            encrypted_vault_key: crate::utils::b64_encode(encrypted_vault_key),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_create_vault(server: Arc<Server>) {
        const VAULT_NAME: &str = "MyTestVault";
        const SHARE_ID: &str = "MyShareID";
        const VAULT_ID: &str = "MyVaultID";

        let client = server.pass_client().await;
        let handled = server.handler("/pass/v1/vault", |_| {
            success(CreateVaultResponse {
                share: CreateVaultResponseContent {
                    share_id: SHARE_ID.to_string(),
                    vault_id: VAULT_ID.to_string(),
                },
            })
        });

        let recorder = server.new_recorder();
        let (share_id, vault_id) = client
            .create_vault(CreateVaultArgs::new(VAULT_NAME.to_string()).unwrap())
            .await
            .expect("Should be able to create the vault request");

        assert_eq!(SHARE_ID, share_id.value());
        assert_eq!(VAULT_ID, vault_id.value());

        assert_hit!(handled);

        let req: CreateVaultRequest = last_request!(recorder);
        // Check vault key is encrypted with user key
        let user_key = client.get_primary_user_key().await.unwrap();
        let (private, public) = user_key.into_keys();

        let encrypted_vault_key = crate::utils::b64_decode(&req.encrypted_vault_key).unwrap();
        let pgp_crypto = client.client_features.get_pgp_crypto().await;
        let decrypted_vault_key = pgp_crypto
            .decrypt_and_verify(encrypted_vault_key, vec![private], vec![public], None)
            .await
            .expect("Error decrypting and verifying vault key");
        assert_eq!(32, decrypted_vault_key.len());

        // Decrypt vault content
        let encrypted_vault_content = crate::utils::b64_decode(&req.content).unwrap();
        let decrypted_vault_content = crypto::decrypt(
            &encrypted_vault_content,
            &decrypted_vault_key,
            EncryptionTag::VaultContent,
        )
        .expect("Error decrypting vault content");

        let parsed = VaultData::deserialize(&decrypted_vault_content)
            .expect("Failed to parse vault content");
        assert_eq!(VAULT_NAME, parsed.name);
    }
}
