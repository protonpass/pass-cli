use crate::PassClient;
use crate::item::item_keys::OpenedItemKey;
use crate::item::list::ItemRevision;
use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use futures::stream::{self, StreamExt};
use pass_domain::{
    Item, ItemData, ItemFlag, ItemId, ItemState, ShareId, ShareType, VaultId, crypto,
};
use std::collections::HashMap;

const MAX_CONCURRENCY: usize = 10;

#[derive(Clone)]
pub(crate) struct ItemWithItemKey {
    pub item: Item,
    pub item_key: OpenedItemKey,
}

impl PassClient {
    pub(crate) async fn open_items(
        &self,
        share_id: &ShareId,
        items: Vec<ItemRevision>,
    ) -> Result<Vec<ItemWithItemKey>> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;

        match share.share_type {
            ShareType::Item { item_id, vault_id } => self
                .open_items_with_item_share(share.id, vault_id, item_id, items)
                .await
                .context("Error opening items with item_share"),
            ShareType::Vault { vault_id } => self
                .open_items_with_vault_share(share.id, vault_id, items)
                .await
                .context("Error opening items with vault_share"),
        }
    }

    /// Get the decrypted item key for an ItemRevision
    ///
    /// For VaultShare: ShareKeys -> get by rotation -> open with open_share_key -> decrypt ItemKey -> return ItemKey
    /// For ItemShare: ShareKeys -> get by rotation -> open with open_share_key -> this IS the ItemKey
    pub(crate) async fn get_item_key(
        &self,
        share_id: &ShareId,
        item: &ItemRevision,
    ) -> Result<OpenedItemKey> {
        self.get_item_key_with_cache(share_id, item, &mut HashMap::new())
            .await
    }

    pub(crate) async fn get_item_key_by_ids(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<OpenedItemKey> {
        let revision = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item revision")?;
        self.get_item_key(share_id, &revision)
            .await
            .context("Error getting item key")
    }

    async fn get_item_key_with_cache(
        &self,
        share_id: &ShareId,
        item: &ItemRevision,
        opened_share_keys: &mut HashMap<u8, Bytes>,
    ) -> Result<OpenedItemKey> {
        let opened_share_key = if let Some(key) = opened_share_keys.get(&item.key_rotation) {
            key.clone()
        } else {
            let share_keys = self.get_share_keys(share_id).await.context(format!(
                "Error retrieving share keys for share {}",
                share_id.value()
            ))?;

            let share_key = share_keys
                .find_by_rotation(item.key_rotation)
                .ok_or_else(|| {
                    anyhow!("Error finding share key for rotation {}", item.key_rotation)
                })?
                .clone();

            let opened_share_key = self
                .open_share_key(share_key)
                .await
                .context("Error opening share key")?;

            let opened_share_key = Bytes::from(opened_share_key.value());
            opened_share_keys.insert(item.key_rotation, opened_share_key.clone());
            opened_share_key
        };

        // Determine if this is a VaultShare or ItemShare based on presence of item_key
        match &item.item_key {
            Some(encrypted_item_key) => {
                // VaultShare: decrypt the item key using the opened share key
                let encrypted_item_key = crate::utils::b64_decode(encrypted_item_key)
                    .context("Error decoding item key")?;

                let decrypted_item_key = crypto::decrypt(
                    &encrypted_item_key,
                    opened_share_key.as_ref(),
                    crypto::EncryptionTag::ItemKey,
                )
                .map_err(|e| {
                    error!("Error decrypting item key: {}", e);
                    anyhow!("Error decrypting item key")
                })?;

                Ok(OpenedItemKey::new(decrypted_item_key, item.key_rotation))
            }
            None => {
                // ItemShare: the opened share key IS the item key
                Ok(OpenedItemKey::new(
                    opened_share_key.to_vec(),
                    item.key_rotation,
                ))
            }
        }
    }

    async fn open_items_with_item_share(
        &self,
        share_id: ShareId,
        vault_id: VaultId,
        item_id: ItemId,
        items: Vec<ItemRevision>,
    ) -> Result<Vec<ItemWithItemKey>> {
        if items.len() != 1 {
            return Err(anyhow!(
                "Item share should grant access to 1 item, and got {}",
                items.len()
            ));
        }

        let item = match items.into_iter().next() {
            Some(item) => item,
            None => return Err(anyhow!("Item list should contain at least one item")),
        };

        let item_state = ItemState::try_from(item.state).context("Error parsing item state")?;

        let item_key = self
            .get_item_key(&share_id, &item)
            .await
            .context("Error getting item key")?;

        let decoded_content =
            crate::utils::b64_decode(&item.content).context("Error decoding item content")?;

        let decrypted = crypto::decrypt(
            &decoded_content,
            item_key.key.as_ref(),
            crypto::EncryptionTag::ItemContent,
        )
        .map_err(|e| {
            error!("Error decrypting item content: {}", e);
            anyhow!("Error decrypting item content")
        })?;

        let parsed = ItemData::deserialize(&decrypted).context("Error parsing item data")?;
        Ok(vec![ItemWithItemKey {
            item: Item {
                id: item_id,
                content: parsed,
                state: item_state,
                share_id,
                vault_id,
                flags: ItemFlag::parse_flags(item.flags),
            },
            item_key,
        }])
    }

    async fn open_items_with_vault_share(
        &self,
        share_id: ShareId,
        vault_id: VaultId,
        items: Vec<ItemRevision>,
    ) -> Result<Vec<ItemWithItemKey>> {
        // Process items concurrently with built-in parallelism limiting
        let results: Vec<(usize, ItemWithItemKey)> = stream::iter(items.into_iter().enumerate())
            .map(|(index, item)| {
                let client = self.clone();
                let share_id = share_id.clone();
                let vault_id = vault_id.clone();
                async move {
                    let item_state = match ItemState::try_from(item.state) {
                        Ok(state) => state,
                        Err(e) => {
                            error!("Error parsing item state for item {}: {}", item.item_id, e);
                            return None;
                        }
                    };

                    let item_key = match client.get_item_key(&share_id, &item).await {
                        Ok(key) => key,
                        Err(e) => {
                            error!("Error getting item key for item {}: {}", item.item_id, e);
                            return None;
                        }
                    };

                    let decoded_content = match crate::utils::b64_decode(&item.content) {
                        Ok(content) => content,
                        Err(e) => {
                            error!(
                                "Error decoding item content for item {}: {}",
                                item.item_id, e
                            );
                            return None;
                        }
                    };

                    let decrypted = match crypto::decrypt(
                        &decoded_content,
                        item_key.key.as_ref(),
                        crypto::EncryptionTag::ItemContent,
                    ) {
                        Ok(content) => content,
                        Err(e) => {
                            error!(
                                "Error decrypting item content for item {}: {}",
                                item.item_id, e
                            );
                            return None;
                        }
                    };

                    let parsed = match ItemData::deserialize(&decrypted) {
                        Ok(data) => data,
                        Err(e) => {
                            error!("Error parsing item data for item {}: {}", item.item_id, e);
                            return None;
                        }
                    };

                    Some((
                        index,
                        ItemWithItemKey {
                            item: Item {
                                id: ItemId::new(item.item_id),
                                content: parsed,
                                state: item_state,
                                share_id,
                                vault_id,
                                flags: ItemFlag::parse_flags(item.flags),
                            },
                            item_key,
                        },
                    ))
                }
            })
            .buffered(MAX_CONCURRENCY)
            .filter_map(|result| async { result })
            .collect()
            .await;

        // Sort results by original index to preserve order
        let mut results_with_index = results;
        results_with_index.sort_by_key(|(index, _)| *index);

        let items: Vec<ItemWithItemKey> = results_with_index
            .into_iter()
            .map(|(_, item)| item)
            .collect();

        Ok(items)
    }
}
