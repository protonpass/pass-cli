use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use pass_domain::LocalKey;
use pass_domain::crypto::EncryptionTag;

impl<C: PassClientContext> PassClient<C> {
    pub async fn encrypt_with_local_key(&self, data: &[u8]) -> Result<Vec<u8>> {
        let local_key = self.get_local_key().await?;
        match pass_domain::crypto::encrypt(data, local_key.as_ref(), EncryptionTag::Unknown) {
            Ok(encrypted_data) => Ok(encrypted_data),
            Err(e) => Err(anyhow!("Error encrypting data: {:?}", e)),
        }
    }

    pub async fn decrypt_with_local_key(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let local_key = self.get_local_key().await?;
        match pass_domain::crypto::decrypt(ciphertext, local_key.as_ref(), EncryptionTag::Unknown) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Error decrypting data: {:?}", e)),
        }
    }

    async fn get_local_key(&self) -> Result<LocalKey> {
        let provider = self
            .client_features
            .get_local_key_provider()
            .await
            .context("Error getting local key provider")?;
        let local_key = provider
            .get_key()
            .await
            .context("Error getting local key")?;

        Ok(local_key)
    }
}
