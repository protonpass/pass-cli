use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core;
use pass_domain::{LockedUserKey, UserKey};
use std::path::Path;

const USER_KEYS_FILE_NAME: &str = "user_keys.enc";

fn api_user_key_to_locked_user_key(value: core::v4::keys::Key) -> LockedUserKey {
    LockedUserKey {
        id: value.id,
        private_key: value.private_key,
        token: value.token,
        signature: value.signature,
        primary: value.primary.into(),
        active: value.active.into(),
    }
}

#[derive(Clone)]
struct UserKeysCacheType;

impl PassClient {
    pub async fn get_user_keys(&self) -> Result<Vec<UserKey>> {
        let client = self.clone();
        self.cache
            .update_if_no_value(UserKeysCacheType, || async move {
                let passphrases = client
                    .get_key_passphrases()
                    .await
                    .context("Error getting key passphrases")?;
                let user_keys = client
                    .load_user_keys()
                    .await
                    .context("Error fetching user keys")?;

                let account_crypto = client.client_features.get_account_crypto().await;

                account_crypto
                    .open_user_keys(user_keys, passphrases.into_map())
                    .await
                    .context("Error opening user keys")
            })
            .await
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

    async fn load_user_keys(&self) -> Result<Vec<LockedUserKey>> {
        match self.load_user_keys_from_fs().await {
            Ok(keys) => {
                if let Some(keys) = keys {
                    return Ok(keys);
                }
            }
            Err(e) => {
                warn!("Error loading cached user keys: {e}");
            }
        }

        let remote_keys = self
            .fetch_user_keys()
            .await
            .context("Error fetching user keys")?;
        let serialized = serde_json::to_vec(&remote_keys).context("Error serializing user keys")?;
        let encrypted = self
            .encrypt_with_local_key(&serialized)
            .await
            .context("Error encrypting user keys")?;
        let fs = self.client_features.get_fs().await;
        fs.store_file(encrypted, Path::new(USER_KEYS_FILE_NAME))
            .await
            .context("Error caching user keys")?;
        Ok(remote_keys)
    }

    async fn load_user_keys_from_fs(&self) -> Result<Option<Vec<LockedUserKey>>> {
        let fs = self.client_features.get_fs().await;
        let path = Path::new(USER_KEYS_FILE_NAME).to_path_buf();
        let has_cached_keys = fs
            .file_exists(&path)
            .await
            .context("Error checking user keys")?;
        if has_cached_keys {
            let contents = fs
                .get_file(path.as_path())
                .await
                .context("Error loading user keys")?;
            let decrypted = self
                .decrypt_with_local_key(&contents)
                .await
                .context("Error decrypting user keys")?;
            let decoded =
                serde_json::from_slice(&decrypted).context("Error deserializing user keys")?;
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    }

    async fn fetch_user_keys(&self) -> Result<Vec<LockedUserKey>> {
        debug!("Fetching user keys");
        let res = self.send(GET!("/core/v4/users")).await?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }
        let res: core::v4::users::GetRes = res.ok()?.into_body_json()?;

        let mapped = res
            .user
            .keys
            .into_iter()
            .map(api_user_key_to_locked_user_key)
            .collect();
        Ok(mapped)
    }
}
