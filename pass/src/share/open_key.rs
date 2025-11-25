use crate::PassClient;
use crate::crypto::share_key::{OpenShareKeyFlow, OpenShareKeyForGroupFlow};
use crate::share::ShareKey;
use anyhow::{Context, Result, anyhow};
use pass_domain::{AddressId, DecryptedShareKey, GroupId, Share, ShareId};

impl PassClient {
    pub(crate) async fn get_all_opened_share_keys(
        &self,
        share_id: &ShareId,
        force_refresh: bool,
    ) -> Result<Vec<DecryptedShareKey>> {
        if !force_refresh {
            // Try to get from database first
            if let Ok(data_storage) = self.client_features.get_data_storage().await {
                let share_key_storage = data_storage.get_share_key_storage().await;

                if let Ok(Some(cached_keys)) = share_key_storage.get_share_keys(share_id).await
                    && !cached_keys.is_empty()
                {
                    trace!(
                        "Using {} cached decrypted share keys from database",
                        cached_keys.len()
                    );
                    return Ok(cached_keys);
                }
            }
        }

        // Not force refresh or not found in cache, fetch encrypted keys and open all of them
        trace!("Share keys not in cache, fetching and opening all");
        let share_keys = self.get_share_keys(share_id).await?;
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;

        let mut decrypted_keys = Vec::with_capacity(share_keys.keys.len());
        for key in share_keys.keys {
            let decrypted = self
                .open_share_key_for_share(&share, key)
                .await
                .context("Error opening share key")?;
            decrypted_keys.push(decrypted);
        }

        // Store in database for future use (best effort, do not fail in case of error)
        if let Ok(data_storage) = self.client_features.get_data_storage().await {
            let share_key_storage = data_storage.get_share_key_storage().await;
            let res = share_key_storage
                .store_share_keys(share_id, decrypted_keys.clone())
                .await;
            if let Err(e) = res {
                warn!("Error storing share keys: {e:#}");
            }
        }

        Ok(decrypted_keys)
    }

    pub(crate) async fn get_opened_share_key_by_rotation(
        &self,
        share_id: &ShareId,
        key_rotation: u8,
    ) -> Result<DecryptedShareKey> {
        let keys = self.get_all_opened_share_keys(share_id, false).await?;
        let key = keys.into_iter().find(|k| k.key_rotation == key_rotation);

        if let Some(key) = key {
            return Ok(key);
        }

        // Not in the cache, get share keys forcing refresh
        trace!(
            "Share key not in cache, re-fetching and opening for rotation {}",
            key_rotation
        );

        let keys = self.get_all_opened_share_keys(share_id, true).await?;
        keys.into_iter()
            .find(|k| k.key_rotation == key_rotation)
            .context("Could not find share key for rotation")
    }

    pub(crate) async fn open_share_key_for_share_id(
        &self,
        share_id: &ShareId,
        key: ShareKey,
    ) -> Result<DecryptedShareKey> {
        // Try to get from database first
        if let Ok(data_storage) = self.client_features.get_data_storage().await {
            let share_key_storage = data_storage.get_share_key_storage().await;

            if let Ok(Some(cached_keys)) = share_key_storage.get_share_keys(share_id).await {
                // Find the key with matching rotation
                if let Some(cached_key) = cached_keys
                    .into_iter()
                    .find(|k| k.key_rotation == key.key_rotation)
                {
                    trace!(
                        "Using cached decrypted share key from database for rotation {}",
                        key.key_rotation
                    );
                    return Ok(cached_key);
                }
            }
        }

        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        self.open_share_key_for_share(&share, key).await
    }

    pub(crate) async fn open_share_key_for_share(
        &self,
        share: &Share,
        key: ShareKey,
    ) -> Result<DecryptedShareKey> {
        // Try to get from database first
        if let Ok(data_storage) = self.client_features.get_data_storage().await {
            let share_key_storage = data_storage.get_share_key_storage().await;

            if let Ok(Some(cached_keys)) = share_key_storage.get_share_keys(&share.id).await {
                // Find the key with matching rotation
                if let Some(cached_key) = cached_keys
                    .into_iter()
                    .find(|k| k.key_rotation == key.key_rotation)
                {
                    trace!(
                        "Using cached decrypted share key from database for rotation {}",
                        key.key_rotation
                    );
                    return Ok(cached_key);
                }
            }
        }

        match share.group_id {
            None => self
                .open_share_key_for_direct_share(key)
                .await
                .context("Error opening ShareKey for Share"),
            Some(ref group_id) => self
                .open_share_key_from_group(&share.address_id, group_id, key)
                .await
                .context("Error opening ShareKey for Share via Group"),
        }
    }

    async fn open_share_key_for_direct_share(&self, key: ShareKey) -> Result<DecryptedShareKey> {
        let uks = self.get_user_keys().await?;
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let flow = OpenShareKeyFlow::new(pgp_crypto, uks);
        let share_key = flow
            .open(key.clone())
            .await
            .context("failed to open ShareKey")?;
        Ok(DecryptedShareKey::new(key.key_rotation, share_key))
    }

    pub(crate) async fn open_share_key_from_group(
        &self,
        address: &AddressId,
        group_id: &GroupId,
        key: ShareKey,
    ) -> Result<DecryptedShareKey> {
        let invited_address = self
            .get_address(address)
            .await
            .context("Failed to get address")?;
        let address_keys = self
            .open_address_keys(invited_address.keys)
            .await
            .context("Failed to open address keys")?;
        let group_addresses = self
            .get_group_addresses()
            .await
            .context("Failed to fetch groups")?;
        let group_address = group_addresses
            .into_iter()
            .find(|g| g.group_id.eq(group_id))
            .ok_or_else(|| anyhow!("Could not find invited group"))?;

        let group_public_keys = self
            .get_keys_for_email(&group_address.address.email, false)
            .await
            .context("Error getting public keys for group")?;

        let pgp_crypto = self.client_features.get_pgp_crypto().await;
        let flow = OpenShareKeyForGroupFlow::new(pgp_crypto, address_keys, group_public_keys);
        let share_key = flow
            .open(key.clone())
            .await
            .context("failed to open ShareKey")?;
        Ok(DecryptedShareKey::new(key.key_rotation, share_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::keys::{
        ActivePublicKeysResponse, AddressDataResponse, PublicAddressKeyResponse,
    };
    use crate::invite::group::keys::{GetGroupsResponse, GroupResponse};
    use crate::share::EncryptedShareKey;
    use crate::test_tools::*;
    use muon::rest::core::v4::{addresses, keys};
    use pass_domain::{DataToArmor, PlainText, PublicKey, ShareRole, ShareType, crypto};

    #[muon::test(scheme(HTTP))]
    async fn open_share_key_for_direct_share(server: Arc<Server>) {
        const SHARE_ID: &str = "SHARE_ID";
        const VAULT_ID: &str = "VAULT_ID";

        let client = server.pass_client().await;

        let share_key_content = crypto::generate_encryption_key();
        let encrypted_share_key = client.encrypt_for_user_key(share_key_content.clone()).await;
        let share_key = ShareKey::new(1, EncryptedShareKey(encrypted_share_key));
        let share = Share {
            id: share_id!(SHARE_ID),
            address_id: address_id!(TEST_ADDRESS_ID),
            share_type: ShareType::Vault {
                vault_id: vault_id!(VAULT_ID),
            },
            vault_id: vault_id!(VAULT_ID),
            permission: Default::default(),
            content: None,
            share_role: ShareRole::Owner,
            group_id: None,
        };
        let opened_share_key = client
            .open_share_key_for_share(&share, share_key)
            .await
            .expect("Should be able to open share key");

        assert_eq!(share_key_content, opened_share_key.value());
    }

    #[muon::test(scheme(HTTP))]
    async fn open_share_key_for_group_share(server: Arc<Server>) {
        const SHARE_ID: &str = "SHARE_ID";
        const VAULT_ID: &str = "VAULT_ID";
        const GROUP_ID: &str = "GROUP_ID";
        const GROUP_ADDRESS_ID: &str = "GROUP_ADDRESS_ID";
        const GROUP_ADDRESS_EMAIL: &str = "group@address.test";
        const GROUP_ADDRESS_KEY_ID: &str = "GROUP_ADDRESS_KEY_ID";

        let client = server.pass_client().await;
        let (group_private_key, group_armored_public_key) = {
            let crypto = client.client_features.get_pgp_crypto().await;
            let (private, public) = crypto
                .generate_key_pair(
                    GROUP_ADDRESS_ID.to_string(),
                    GROUP_ADDRESS_EMAIL.to_string(),
                )
                .await
                .expect("Failed to generate group address key");

            let public_armored = crypto
                .armor(DataToArmor::PublicKey(public.as_ref().to_vec()))
                .await
                .expect("Failed to armor");

            (private, public_armored)
        };

        server.handler_with_method(Method::GET, "/core/v4/groups", move |_| {
            success(GetGroupsResponse {
                groups: vec![GroupResponse {
                    id: GROUP_ID.to_string(),
                    name: "Test group name".to_string(),
                    address: Some(addresses::Address {
                        id: GROUP_ADDRESS_ID.to_string(),
                        email: GROUP_ADDRESS_EMAIL.to_string(),
                        keys: vec![keys::Key {
                            id: GROUP_ADDRESS_KEY_ID.to_string(),
                            private_key: "".to_string(), // Ignored, as we don't use it here
                            token: None,
                            signature: None,
                            primary: Default::default(),
                            active: Default::default(),
                        }],
                    }),
                    permissions: 0,
                    create_time: 0,
                    flags: 0,
                    group_visibility: 0,
                    member_visibility: 0,
                    description: "".to_string(),
                }],
            })
        });

        let group_armored_public_key_clone = group_armored_public_key.clone();
        server.handler_with_method(Method::GET, "/core/v4/keys/all", move |_| {
            success(ActivePublicKeysResponse {
                address: AddressDataResponse {
                    keys: vec![PublicAddressKeyResponse {
                        public_key: group_armored_public_key_clone.to_string(),
                        primary: 1,
                    }],
                },
            })
        });

        let share_key_raw = crypto::generate_encryption_key();

        // Share keys via group are encrypted for the primary address key and signed with the group
        // key.
        let encrypted_share_key = {
            let pgp = client.client_features.get_pgp_crypto().await;
            let address_public_key = pgp
                .unarmor(TEST_ADDRESS_KEY_PUBLIC_KEY.to_string())
                .await
                .expect("Failed to get unarmored address key");

            pgp.encrypt_and_sign(
                PlainText::new(share_key_raw.clone()),
                PublicKey::new(address_public_key),
                group_private_key.clone(),
                None,
            )
            .await
            .expect("Failed to encrypt share key")
        };

        let share_key = ShareKey::new(1, EncryptedShareKey(encrypted_share_key.clone()));
        let share = Share {
            id: share_id!(SHARE_ID),
            address_id: address_id!(TEST_ADDRESS_ID),
            share_type: ShareType::Vault {
                vault_id: vault_id!(VAULT_ID),
            },
            vault_id: vault_id!(VAULT_ID),
            permission: Default::default(),
            content: None,
            share_role: ShareRole::Owner,
            group_id: Some(group_id!(GROUP_ID)),
        };
        let opened_share_key = client
            .open_share_key_for_share(&share, share_key)
            .await
            .expect("Should be able to open share key");

        assert_eq!(share_key_raw, opened_share_key.value());
    }
}
