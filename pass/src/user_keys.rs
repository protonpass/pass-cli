use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core;
use pass_domain::{LockedUserKey, UserKey};

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
                    .fetch_user_keys()
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

    async fn fetch_user_keys(&self) -> Result<Vec<LockedUserKey>> {
        debug!("Fetching user keys");
        let res = self.client.send(GET!("/core/v4/users")).await?;
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
