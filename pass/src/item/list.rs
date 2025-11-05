use crate::PassClient;
use crate::item::open::ItemWithItemKey;
use crate::pagination::SincePagination;
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::{Item, ShareId};
use std::collections::HashMap;

struct ItemsForShareCacheType;
type ItemsForShareCache = HashMap<ShareId, SerializedItemsWithKey>;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct SerializedItemsWithKey(Vec<u8>);

impl SerializedItemsWithKey {
    pub fn new(content: &[ItemWithItemKey], xor_key: u8) -> Result<Self> {
        let serialized = serde_json::to_vec(content).context("Error serializing content")?;
        let xored = Self::perform_xor(serialized, xor_key);
        Ok(Self(xored))
    }

    pub fn deserialize(&self, xor_key: u8) -> Result<Vec<ItemWithItemKey>> {
        let xored = Self::perform_xor(self.0.clone(), xor_key);
        let deserialized = serde_json::from_slice(&xored).context("Error deserializing content")?;
        Ok(deserialized)
    }

    fn perform_xor(mut data: Vec<u8>, xor_key: u8) -> Vec<u8> {
        for i in &mut data {
            *i ^= xor_key;
        }
        data
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct GetItemsResponse {
    #[serde(rename = "Items")]
    pub items: ItemsList,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ItemsList {
    #[serde(rename = "RevisionsData")]
    pub revisions: Vec<ItemRevision>,
    #[serde(rename = "LastToken")]
    pub last_token: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[allow(dead_code)]
pub(crate) struct ItemRevision {
    #[serde(rename = "ItemID")]
    pub item_id: String,
    #[serde(rename = "Revision")]
    pub revision: u64,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: i32,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ItemKey")]
    pub item_key: Option<String>,
    #[serde(rename = "State")]
    pub state: u8,
    #[serde(rename = "Flags")]
    pub flags: u64,
    #[serde(rename = "AliasEmail")]
    pub alias_email: Option<String>,
}

impl PassClient {
    pub async fn list_items(&self, share_id: &ShareId) -> Result<Vec<Item>> {
        {
            let share_id = share_id.clone();
            self.cache
                .ensure_has_value(ItemsForShareCacheType, ItemsForShareCache::new)
                .await;

            let cached: Option<ItemsForShareCache> = self.cache.get(ItemsForShareCacheType).await;
            if let Some(cached) = cached {
                let share_items: Option<&SerializedItemsWithKey> = cached.get(&share_id);
                if let Some(cached_items) = share_items {
                    match cached_items.deserialize(self.memory_xor_key) {
                        Ok(items) => {
                            let items = items.into_iter().map(|i| i.item).collect();
                            trace!("Returning cached items for share {share_id}");
                            return Ok(items);
                        }
                        Err(e) => {
                            warn!("Error deserializing cached items: {e:#}");
                        }
                    }
                }
            }
        }

        let items = self
            .fetch_items(share_id)
            .await
            .context("Error fetching items")?;

        let opened = if items.is_empty() {
            vec![]
        } else {
            self.open_items(share_id, items)
                .await
                .context("Error opening items")?
        };

        let serialized = SerializedItemsWithKey::new(&opened, self.memory_xor_key)
            .context("Error caching items")?;
        self.cache
            .update(
                ItemsForShareCacheType,
                |share_items: &mut ItemsForShareCache| {
                    share_items.insert(share_id.clone(), serialized);
                },
            )
            .await;

        let items = opened.into_iter().map(|i| i.item).collect();
        Ok(items)
    }

    async fn fetch_items(&self, share_id: &ShareId) -> Result<Vec<ItemRevision>> {
        let mut items = Vec::new();
        let mut pagination = SincePagination::default();
        loop {
            let mut req = GET!("/pass/v1/share/{}/item", share_id)
                .query(("PageSize".to_string(), format!("{}", pagination.page_size)));

            if let Some(ref since) = pagination.since {
                req = req.query(("Since".to_string(), since.to_string()));
            }

            let res = self.send(req).await.context("Error fetching items page")?;

            let response: GetItemsResponse = assert_response!(res);
            let response_content = response.items;
            let revisions = response_content.revisions;

            trace!("Retrieved {} items", revisions.len());
            if !revisions.is_empty() {
                let retrieved_size = revisions.len();
                items.extend(revisions);
                if retrieved_size < pagination.page_size {
                    break;
                }

                match pagination.next(response_content.last_token) {
                    Some(p) => pagination = p,
                    None => break,
                }
            } else {
                break;
            }
        }

        trace!(
            "Finished item retrieval process. Retrieved {} items",
            items.len()
        );

        Ok(items)
    }

    pub(crate) async fn clear_items_cache(&self, share_id: &ShareId) {
        self.cache
            .update(ItemsForShareCacheType, |cache: &mut ItemsForShareCache| {
                cache.remove(share_id);
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use pass_domain::{Item, ItemState};
    use std::sync::{Arc, atomic::AtomicBool};

    fn setup_list_items_endpoint(
        server: &Arc<Server>,
        share_id: &str,
        revisions: Vec<ItemRevision>,
    ) -> Arc<AtomicBool> {
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/item", share_id),
            move |_| {
                success(GetItemsResponse {
                    items: ItemsList {
                        revisions: revisions.clone(),
                        last_token: None,
                    },
                })
            },
        )
    }

    #[test]
    fn test_serialized_items_with_key_round_trip() {
        const XOR_KEY: u8 = 0x42;

        // Create test item data
        let item_data = create_random_item();
        let encrypted_data = encrypt_item_contents(item_data.clone());

        // Create an ItemWithItemKey
        let item_with_key = ItemWithItemKey {
            item: Item {
                id: item_id!("test_item_id"),
                share_id: share_id!("test_share_id"),
                vault_id: vault_id!("test_vault_id"),
                content: item_data,
                state: ItemState::Active,
                flags: vec![],
            },
            item_key: crate::item::item_keys::OpenedItemKey {
                key_rotation: 1,
                key: encrypted_data.item_key,
            },
        };

        let items = vec![item_with_key.clone()];

        // Serialize with XOR key
        let serialized =
            SerializedItemsWithKey::new(&items, XOR_KEY).expect("Should be able to serialize");

        // Deserialize with same XOR key
        let deserialized = serialized
            .deserialize(XOR_KEY)
            .expect("Should be able to deserialize");

        // Verify round-trip works
        assert_eq!(1, deserialized.len());
        assert_eq!(
            item_with_key.item.id.value(),
            deserialized[0].item.id.value()
        );
        assert_eq!(
            item_with_key.item.share_id.value(),
            deserialized[0].item.share_id.value()
        );
        assert_eq!(
            item_with_key.item_key.key_rotation,
            deserialized[0].item_key.key_rotation
        );
    }

    #[test]
    fn test_serialized_items_with_key_wrong_key_fails() {
        const XOR_KEY: u8 = 0x42;
        const WRONG_KEY: u8 = 0x99;

        // Create test item data
        let item_data = create_random_item();
        let encrypted_data = encrypt_item_contents(item_data.clone());

        let item_with_key = ItemWithItemKey {
            item: Item {
                id: item_id!("test_item_id"),
                share_id: share_id!("test_share_id"),
                vault_id: vault_id!("test_vault_id"),
                content: item_data,
                state: ItemState::Active,
                flags: vec![],
            },
            item_key: crate::item::item_keys::OpenedItemKey {
                key_rotation: 1,
                key: encrypted_data.item_key,
            },
        };

        let items = vec![item_with_key];

        // Serialize with one key
        let serialized =
            SerializedItemsWithKey::new(&items, XOR_KEY).expect("Should be able to serialize");

        // Try to deserialize with wrong key - should fail
        let result = serialized.deserialize(WRONG_KEY);
        assert!(
            result.is_err(),
            "Deserialization with wrong key should fail"
        );
    }

    #[test]
    fn test_serialized_items_with_key_empty_list() {
        const XOR_KEY: u8 = 0x42;

        let items: Vec<ItemWithItemKey> = vec![];

        // Serialize empty list
        let serialized = SerializedItemsWithKey::new(&items, XOR_KEY)
            .expect("Should be able to serialize empty list");

        // Deserialize
        let deserialized = serialized
            .deserialize(XOR_KEY)
            .expect("Should be able to deserialize empty list");

        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_serialized_items_xor_actually_encrypts() {
        const XOR_KEY: u8 = 0x42;

        // Create test item data
        let item_data = create_random_item();
        let encrypted_data = encrypt_item_contents(item_data.clone());

        let item_with_key = ItemWithItemKey {
            item: Item {
                id: item_id!("test_item_id"),
                share_id: share_id!("test_share_id"),
                vault_id: vault_id!("test_vault_id"),
                content: item_data,
                state: ItemState::Active,
                flags: vec![],
            },
            item_key: crate::item::item_keys::OpenedItemKey {
                key_rotation: 1,
                key: encrypted_data.item_key,
            },
        };

        let items = vec![item_with_key];

        // Get original serialized data without XOR
        let original_json = serde_json::to_vec(&items).expect("Should serialize to JSON");

        // Serialize with XOR key
        let serialized =
            SerializedItemsWithKey::new(&items, XOR_KEY).expect("Should be able to serialize");

        // Verify that the XOR'd data is different from the original
        assert_ne!(
            &original_json, &serialized.0,
            "XOR encrypted data should be different from original"
        );

        // Verify that all bytes are XOR'd
        for (orig_byte, xor_byte) in original_json.iter().zip(serialized.0.iter()) {
            assert_eq!(orig_byte ^ XOR_KEY, *xor_byte, "Each byte should be XOR'd");
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_items_empty_response(server: Arc<Server>) {
        const SHARE_ID: &str = "TestShareID1";

        let client = server.pass_client().await;

        // Setup vault share and keys
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Setup empty items list
        let handled = setup_list_items_endpoint(&server, SHARE_ID, vec![]);

        let recorder = server.new_recorder();
        let items = client
            .list_items(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to list items");

        assert_hit!(handled);
        let requests = recorder.read();

        // Only request items. As it's empty, the share is not requested
        assert_eq!(requests.len(), 1);
        let req = requests.iter().next().unwrap();
        assert_eq!(
            format!("/pass/v1/share/{}/item", SHARE_ID),
            req.uri().path()
        );

        assert!(items.is_empty(), "Expected empty items list");
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_items_two_shares_with_one_item_each(server: Arc<Server>) {
        const SHARE_ID_1: &str = "TestShareID1";
        const SHARE_ID_2: &str = "TestShareID2";
        const ITEM_ID_1: &str = "Item1";
        const ITEM_ID_2: &str = "Item2";

        let client = server.pass_client().await;

        // Setup first share
        setup_vault_share(&server, SHARE_ID_1);
        setup_share_keys(&server, SHARE_ID_1);

        // Setup second share
        setup_vault_share(&server, SHARE_ID_2);
        setup_share_keys(&server, SHARE_ID_2);

        // Create item data for first share
        let item_data_1 = create_random_item();
        let encrypted_data_1 = encrypt_item_contents(item_data_1.clone());
        let revision_1 = ItemRevisionBuilder::new(ITEM_ID_1.to_string())
            .with_content(crate::utils::b64_encode(
                &encrypted_data_1.encrypted_contents,
            ))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data_1.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Create item data for second share
        let item_data_2 = create_random_item();
        let encrypted_data_2 = encrypt_item_contents(item_data_2.clone());
        let revision_2 = ItemRevisionBuilder::new(ITEM_ID_2.to_string())
            .with_content(crate::utils::b64_encode(
                &encrypted_data_2.encrypted_contents,
            ))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data_2.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Setup endpoints
        let handled_1 = setup_list_items_endpoint(&server, SHARE_ID_1, vec![revision_1]);
        let handled_2 = setup_list_items_endpoint(&server, SHARE_ID_2, vec![revision_2]);

        // Fetch items for first share
        let items_1 = client
            .list_items(&share_id!(SHARE_ID_1))
            .await
            .expect("Should be able to list items for share 1");

        assert_hit!(handled_1);
        assert_eq!(1, items_1.len(), "Expected 1 item in share 1");
        assert_eq!(ITEM_ID_1, items_1[0].id.value());
        assert_eq!(item_data_1.title, items_1[0].content.title);

        // Fetch items for second share
        let items_2 = client
            .list_items(&share_id!(SHARE_ID_2))
            .await
            .expect("Should be able to list items for share 2");

        assert_hit!(handled_2);
        assert_eq!(1, items_2.len(), "Expected 1 item in share 2");
        assert_eq!(ITEM_ID_2, items_2[0].id.value());
        assert_eq!(item_data_2.title, items_2[0].content.title);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_items_cache_is_used(server: Arc<Server>) {
        const SHARE_ID: &str = "TestShareID1";
        const ITEM_ID: &str = "Item1";

        let client = server.pass_client().await;

        // Setup vault share and keys
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Create item data
        let item_data = create_random_item();
        let encrypted_data = encrypt_item_contents(item_data.clone());
        let revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_data.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Setup endpoint
        let handled = setup_list_items_endpoint(&server, SHARE_ID, vec![revision]);

        let recorder = server.new_recorder();

        // First fetch - should hit the endpoint
        let items_1 = client
            .list_items(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to list items (first call)");

        assert_hit!(handled);
        assert_eq!(1, items_1.len());
        assert_eq!(ITEM_ID, items_1[0].id.value());

        let requests_after_first = recorder.read().len();
        assert!(requests_after_first > 0, "First call should make requests");

        // Second fetch - should use cache, no new requests
        let items_2 = client
            .list_items(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to list items (second call)");

        let requests_after_second = recorder.read().len();
        assert_eq!(
            requests_after_first, requests_after_second,
            "Second call should not make new requests (cache should be used)"
        );

        // Verify the items are the same
        assert_eq!(1, items_2.len());
        assert_eq!(ITEM_ID, items_2[0].id.value());
        assert_eq!(item_data.title, items_2[0].content.title);
    }
}
