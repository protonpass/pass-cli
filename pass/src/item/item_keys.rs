use crate::PassClient;
use crate::share::{EncryptedShareKey, ShareKey};
use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use muon::GET;
use pass_domain::{ItemId, ShareId, ShareType, crypto};
use std::collections::HashMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug)]
pub(crate) struct ItemKeys {
    keys: Vec<ItemKey>,
}

impl ItemKeys {
    pub fn new(keys: Vec<ItemKey>) -> Self {
        Self { keys }
    }
}

#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub(crate) struct EncryptedItemKey(pub(crate) Vec<u8>);

impl AsRef<[u8]> for EncryptedItemKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub(crate) struct ItemKey {
    pub(crate) key: EncryptedItemKey,
    pub(crate) key_rotation: u8,
}

impl ItemKey {
    pub fn new(key: Vec<u8>, key_rotation: u8) -> Self {
        Self {
            key: EncryptedItemKey(key),
            key_rotation,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Zeroize, ZeroizeOnDrop)]
pub(crate) struct DecryptedItemKey(pub(crate) Vec<u8>);

impl DecryptedItemKey {
    pub fn value(self) -> Vec<u8> {
        self.0.clone()
    }
}

impl AsRef<[u8]> for DecryptedItemKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, ZeroizeOnDrop)]
pub(crate) struct OpenedItemKey {
    pub(crate) key: DecryptedItemKey,
    pub(crate) key_rotation: u8,
}

impl OpenedItemKey {
    pub fn new(key: Vec<u8>, key_rotation: u8) -> Self {
        Self {
            key: DecryptedItemKey(key),
            key_rotation,
        }
    }
}

pub(crate) struct OpenedItemKeys {
    pub(crate) keys: Vec<OpenedItemKey>,
}

impl OpenedItemKeys {
    pub fn new(keys: Vec<OpenedItemKey>) -> Self {
        Self { keys }
    }

    pub fn latest(&self) -> Option<&OpenedItemKey> {
        self.keys.iter().max_by_key(|key| key.key_rotation)
    }

    pub fn latest_or_err(&self) -> Result<&OpenedItemKey> {
        self.latest()
            .ok_or(anyhow!("Could not get latest item key"))
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct GetItemKeysResponse {
    #[serde(rename = "Code")]
    pub code: i32,
    #[serde(rename = "Keys")]
    pub keys: ItemKeysResponse,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ItemKeysResponse {
    #[serde(rename = "Keys")]
    pub keys: Vec<ItemKeyResponse>,
    #[serde(rename = "Total")]
    pub total: u32,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ItemKeyResponse {
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "Key")]
    pub key: String,
}

impl PassClient {
    pub(crate) async fn get_item_keys(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<ItemKeys> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;

        match share.share_type {
            // If share is of type vault, we request the item keys
            ShareType::Vault { .. } => {
                let res = self
                    .send(GET!("/pass/v1/share/{share_id}/item/{item_id}/key"))
                    .await
                    .context("Error sending get item keys request")?;

                let response: GetItemKeysResponse = assert_response!(res);

                let mut res = Vec::new();
                for key in response.keys.keys {
                    let decoded =
                        crate::utils::b64_decode(&key.key).context("Error decoding item key")?;
                    res.push(ItemKey::new(decoded, key.key_rotation));
                }

                Ok(ItemKeys::new(res))
            }

            // If share is of type item, share keys are directly user keys
            ShareType::Item { .. } => {
                let share_keys = self
                    .get_share_keys(share_id)
                    .await
                    .context("Error getting share keys")?;

                let mut res = Vec::new();
                for key in share_keys.keys {
                    res.push(ItemKey::new(key.key.clone().value(), key.key_rotation));
                }

                Ok(ItemKeys::new(res))
            }
        }
    }

    pub(crate) async fn open_item_keys(
        &self,
        share_id: &ShareId,
        item_keys: ItemKeys,
    ) -> Result<Vec<OpenedItemKey>> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;

        match share.share_type {
            ShareType::Vault { .. } => self
                .open_item_keys_with_vault_share(share_id, item_keys)
                .await
                .context("Error opening item keys with vault share"),
            ShareType::Item { .. } => self
                .open_item_keys_with_item_share(share_id, item_keys)
                .await
                .context("Error opening item keys with item share"),
        }
    }

    async fn open_item_keys_with_vault_share(
        &self,
        share_id: &ShareId,
        item_keys: ItemKeys,
    ) -> Result<Vec<OpenedItemKey>> {
        let mut res = Vec::with_capacity(item_keys.keys.len());
        let mut cache: HashMap<u8, Bytes> = HashMap::new();
        for key in item_keys.keys {
            let opened_share_key = if let Some(key) = cache.get(&key.key_rotation) {
                key.clone()
            } else {
                // Use the optimized method that checks DB first before fetching from API
                let opened_share_key = self
                    .get_opened_share_key_by_rotation(share_id, key.key_rotation)
                    .await
                    .context("Error getting opened share key")?;

                let opened_share_key = Bytes::from(opened_share_key.value());
                cache.insert(key.key_rotation, opened_share_key.clone());
                opened_share_key
            };

            let decrypted_item_key = crypto::decrypt(
                &key.key.0,
                opened_share_key.as_ref(),
                crypto::EncryptionTag::ItemKey,
            )
            .map_err(|e| {
                error!("Error decrypting item key: {}", e);
                anyhow!("Error decrypting item key")
            })?;

            res.push(OpenedItemKey {
                key: DecryptedItemKey(decrypted_item_key),
                key_rotation: key.key_rotation,
            });
        }

        Ok(res)
    }

    async fn open_item_keys_with_item_share(
        &self,
        share_id: &ShareId,
        item_keys: ItemKeys,
    ) -> Result<Vec<OpenedItemKey>> {
        let mut res = Vec::with_capacity(item_keys.keys.len());

        for key in item_keys.keys {
            let opened = self
                .open_share_key_for_share_id(
                    share_id,
                    ShareKey {
                        key_rotation: key.key_rotation,
                        key: EncryptedShareKey(key.key.0.clone()),
                    },
                )
                .await
                .context("Error opening item key")?;

            res.push(OpenedItemKey {
                key: DecryptedItemKey(opened.value()),
                key_rotation: key.key_rotation,
            })
        }

        Ok(res)
    }
}
