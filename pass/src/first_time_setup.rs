use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use zeroize::{Zeroize, ZeroizeOnDrop};

const PERSONAL_ACCESS_TOKEN_KEY_FILE_NAME: &str = "pat_key";

#[derive(Zeroize, ZeroizeOnDrop)]
pub enum FirstTimeSetupKey {
    Passphrase(Vec<u8>),
    UserPassword(String),
    PersonalAccessToken(Vec<u8>),
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn perform_first_time_setup(&self, pass: &str) -> Result<()> {
        self.setup_key_passphrases(pass)
            .await
            .context("Error setting up key passphrases")?;

        Ok(())
    }

    pub async fn perform_first_time_setup_with_key(&self, key: FirstTimeSetupKey) -> Result<()> {
        match key {
            FirstTimeSetupKey::Passphrase(ref passphrase) => {
                self.setup_key_passphrases_with_passphrase(passphrase)
                    .await
                    .context("Error setting up key passphrases")?;
                Ok(())
            }
            FirstTimeSetupKey::UserPassword(ref user_pass) => {
                self.perform_first_time_setup(user_pass.as_str()).await
            }
            FirstTimeSetupKey::PersonalAccessToken(ref pat_key) => {
                self.setup_personal_access_token_key(pat_key)
                    .await
                    .context("Error setting up Personal Access Token key")?;
                Ok(())
            }
        }
    }

    async fn setup_personal_access_token_key(
        &self,
        personal_access_toekn_key: &[u8],
    ) -> Result<()> {
        use std::path::Path;

        let local_key_provider = self.get_key_provider().await?;
        let local_key = local_key_provider.get_key().await?;

        // Encrypt the personal access token key with the local key
        let encrypted_key = pass_domain::crypto::encrypt(
            personal_access_toekn_key,
            local_key.as_ref(),
            pass_domain::crypto::EncryptionTag::PersonalAccessTokenKey,
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Error encrypting personal access token key with local key: {:?}",
                e
            )
        })?;

        // Store the encrypted personal access token key
        let fs = self.client_features.get_fs().await;
        fs.store_file(
            encrypted_key,
            Path::new(PERSONAL_ACCESS_TOKEN_KEY_FILE_NAME),
        )
        .await
        .context("Error storing personal access token key")?;

        Ok(())
    }

    pub async fn get_local_personal_access_token_key(&self) -> Result<Vec<u8>> {
        use std::path::Path;

        let fs = self.client_features.get_fs().await;
        let encrypted_key = fs
            .get_file(Path::new(PERSONAL_ACCESS_TOKEN_KEY_FILE_NAME))
            .await
            .context("Error loading personal access token key")?;

        let local_key_provider = self.get_key_provider().await?;
        let local_key = local_key_provider.get_key().await?;

        let decrypted_key = pass_domain::crypto::decrypt(
            &encrypted_key,
            local_key.as_ref(),
            pass_domain::crypto::EncryptionTag::PersonalAccessTokenKey,
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Error decrypting personal access token key with local key: {:?}",
                e
            )
        })?;

        Ok(decrypted_key)
    }
}
