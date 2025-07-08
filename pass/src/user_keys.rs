use crate::{PassClient, PrivateKey, PublicKey};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core;
use std::path::Path;

const USER_KEYS_FILE_NAME: &str = "user.keys";

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct UserKey {
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
}

impl UserKey {
    pub fn into_keys(self) -> (PrivateKey, PublicKey) {
        (
            PrivateKey {
                content: self.private_key,
            },
            PublicKey {
                content: self.public_key,
            },
        )
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct SerializableUserKeys {
    keys: Vec<UserKey>,
}

impl PassClient {
    pub async fn setup_user_keys(&self, pass: &str) -> Result<()> {
        let user_keys = self
            .fetch_user_keys(pass)
            .await
            .context("Error fetching user keys")?;
        let serializable = SerializableUserKeys { keys: user_keys };
        let serialized =
            serde_json::to_string(&serializable).context("Error serializing user keys")?;
        let encrypted = self
            .encrypt_with_local_key(serialized.as_bytes())
            .await
            .context("error encrypting user keys")?;

        self.client_features
            .store_file(encrypted, Path::new(USER_KEYS_FILE_NAME))
            .await
            .context("Error storing user keys")?;

        Ok(())
    }

    pub async fn get_user_keys(&self) -> Result<Vec<UserKey>> {
        let keys_file = Path::new(USER_KEYS_FILE_NAME);
        if !self.client_features.file_exists(keys_file).await? {
            return Err(anyhow!(
                "Could not file user keys file at {}",
                keys_file.display()
            ));
        }

        let contents = self
            .client_features
            .get_file(keys_file)
            .await
            .context("Error reading user keys")?;
        let decrypted = self
            .decrypt_with_local_key(&contents)
            .await
            .context("Error decrypting user keys")?;
        let deserialized: SerializableUserKeys =
            serde_json::from_slice(&decrypted).context("Error deserializing user keys")?;

        Ok(deserialized.keys)
    }

    pub(crate) async fn get_primary_user_key(&self) -> Result<UserKey> {
        let mut keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;
        if let Some(key) = keys.pop() {
            Ok(key)
        } else {
            Err(anyhow!("Empty list of user keys"))
        }
    }

    async fn fetch_user_keys(&self, pass: &str) -> Result<Vec<UserKey>> {
        debug!("Fetching user data");
        let res = self.client.send(GET!("/core/v4/users")).await?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }
        let res: core::v4::users::GetRes = res.ok()?.into_body_json()?;
        let user = res.user;

        debug!("Fetching key salts");
        let passphrases = self
            .setup_key_passphrases(pass)
            .await
            .context("Error setting up key salts")?;

        info!("Opening user keys");
        let res = self
            .client_features
            .open_user_keys(user.keys, passphrases.into_map())
            .await?;
        info!("User keys opened ({})", res.len());
        Ok(res)
    }
}
