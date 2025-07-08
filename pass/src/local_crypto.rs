use crate::PassClient;
use anyhow::{Result, anyhow};
use pass_domain::crypto::EncryptionTag;

impl PassClient {
    pub async fn encrypt_with_local_key(&self, data: &[u8]) -> Result<Vec<u8>> {
        let local_key = self.get_local_key().await?;
        match pass_domain::crypto::encrypt(data, &local_key, EncryptionTag::Unknown) {
            Ok(encrypted_data) => Ok(encrypted_data),
            Err(e) => Err(anyhow!("Error encrypting data: {:?}", e)),
        }
    }

    pub async fn decrypt_with_local_key(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let local_key = self.get_local_key().await?;
        match pass_domain::crypto::decrypt(ciphertext, &local_key, EncryptionTag::Unknown) {
            Ok(data) => Ok(data),
            Err(e) => Err(anyhow!("Error decrypting data: {:?}", e)),
        }
    }

    async fn get_local_key(&self) -> Result<Vec<u8>> {
        self.client_features.get_local_key().await
    }
}
