/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use muon::rest::core::v4::keys::Key;
use pass_domain::{AccountType, LockedUserKey, UserKey};
use std::path::Path;

const USER_KEYS_FILE_NAME: &str = "user_keys.enc";
const PERSONAL_ACCESS_TOKEN_ERROR: &str =
    "Personal access tokens and agent sessions cannot perform user key operations";

fn api_user_key_to_locked_user_key(value: Key) -> LockedUserKey {
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

#[derive(Debug, serde::Deserialize)]
struct GetUserKeysResponse {
    #[serde(rename = "User")]
    pub user: UserKeysResponse,
}

#[derive(Debug, serde::Deserialize)]
struct UserKeysResponse {
    #[serde(rename = "Keys")]
    pub keys: Vec<Key>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn get_user_keys(&self) -> Result<Vec<UserKey>> {
        if self.account_type() == AccountType::PersonalAccessToken
            || self.account_type() == AccountType::AgentSession
        {
            return Err(anyhow!(PERSONAL_ACCESS_TOKEN_ERROR));
        }

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
        if self.account_type() == AccountType::PersonalAccessToken
            || self.account_type() == AccountType::AgentSession
        {
            return Err(anyhow!(PERSONAL_ACCESS_TOKEN_ERROR));
        }

        let keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;
        if let Some(key) = keys.first().cloned() {
            Ok(key)
        } else {
            Err(anyhow!("Empty list of user keys"))
        }
    }

    pub(crate) async fn load_user_keys(&self) -> Result<Vec<LockedUserKey>> {
        if self.account_type() == AccountType::PersonalAccessToken
            || self.account_type() == AccountType::AgentSession
        {
            return Err(anyhow!(PERSONAL_ACCESS_TOKEN_ERROR));
        }

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
        let response: GetUserKeysResponse = assert_response!(res);

        let mapped = response
            .user
            .keys
            .into_iter()
            .map(api_user_key_to_locked_user_key)
            .collect();
        Ok(mapped)
    }
}
