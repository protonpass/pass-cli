use crate::{ApiKey, PassClient, PrivateKey, PublicKey};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core;

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
                    .fetch_user_keys()
                    .await
                    .context("Error fetching user keys")?;
                client
                    .client_features
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

    async fn fetch_user_keys(&self) -> Result<Vec<ApiKey>> {
        debug!("Fetching user keys");
        let res = self.client.send(GET!("/core/v4/users")).await?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }
        let res: core::v4::users::GetRes = res.ok()?.into_body_json()?;
        let user = res.user;
        Ok(user.keys)
    }
}
