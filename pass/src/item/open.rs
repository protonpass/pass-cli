use crate::PassClient;
use crate::item::item_keys::OpenedItemKey;
use crate::item::list::ItemRevision;
use anyhow::{Context, Result, anyhow};
use bytes::Bytes;
use chrono::{DateTime, NaiveDateTime};
use futures::stream::{self, StreamExt};
use pass_domain::{
    Item, ItemData, ItemFlag, ItemId, ItemState, ItemType, ShareId, ShareType, VaultId, crypto,
};
use std::collections::HashMap;

const MAX_CONCURRENCY: usize = 10;

fn timestamp_to_naive_datetime(timestamp: u64) -> NaiveDateTime {
    DateTime::from_timestamp(timestamp as i64, 0)
        .map(|dt| dt.naive_utc())
        .unwrap_or_default()
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
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

    pub(crate) async fn get_item_type(
        &self,
        share_id: &ShareId,
        item_revision: &ItemRevision,
    ) -> Result<ItemType> {
        let opened = self
            .open_items(share_id, vec![item_revision.clone()])
            .await
            .context("Error opening item")?;
        if let Some(opened) = opened.first() {
            Ok(ItemType::from_content(&opened.item.content.content))
        } else {
            Err(anyhow::anyhow!("Error getting item type"))
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
            // Use the optimized method that checks DB first before fetching from API
            let opened_share_key = self
                .get_opened_share_key_by_rotation(share_id, item.key_rotation)
                .await
                .context("Error getting opened share key")?;

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
                create_time: timestamp_to_naive_datetime(item.create_time),
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
                                id: ItemId::new(item.item_id.clone()),
                                content: parsed,
                                state: item_state,
                                share_id,
                                vault_id,
                                flags: ItemFlag::parse_flags(item.flags),
                                create_time: timestamp_to_naive_datetime(item.create_time),
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

#[cfg(test)]
mod tests {

    use crate::account::keys::{
        ActivePublicKeysResponse, AddressDataResponse, PublicAddressKeyResponse,
    };
    use crate::invite::group::keys::{GetGroupsResponse, GroupResponse};
    use crate::share::keys::{GetShareKeysResponse, ShareKeyList, ShareKeyResponse};
    use crate::share::list::ShareResponse;
    use crate::test_tools::*;
    use muon::rest::core::v4::{addresses, keys};
    use muon::test::server::{HTTP, Server};
    use pass_domain::{
        CustomItem, CustomSection, DataToArmor, ItemContent, ItemData, ItemExtraField,
        ItemExtraFieldContent, ItemFlag, ItemState, PlainText, PublicKey, TargetType, crypto,
    };
    use std::sync::Arc;

    // Helper function to setup item share (target_type: Item)
    fn setup_item_share(server: &Arc<Server>, share_id: &str, item_id: &str, vault_id: &str) {
        let share_response = ShareResponse {
            share_id: share_id.to_string(),
            address_id: TEST_ADDRESS_ID.to_string(),
            vault_id: vault_id.to_string(),
            target_type: TargetType::Item.value(),
            target_id: item_id.to_string(),
            owner: false,
            permission: 0,
            share_role_id: "1".to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 0,
            group_id: None,
        };
        let share_response_clone = share_response.clone();
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}", share_id),
            move |_| success(share_response_clone.clone()),
        );
        let share_response_clone2 = share_response.clone();
        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(crate::share::list::GetSharesResponse {
                shares: vec![share_response_clone2.clone()],
            })
        });
    }

    // Helper function to setup item share with group
    fn setup_item_share_with_group(
        server: &Arc<Server>,
        share_id: &str,
        item_id: &str,
        vault_id: &str,
        group_id: &str,
    ) {
        let share_response = ShareResponse {
            share_id: share_id.to_string(),
            address_id: TEST_ADDRESS_ID.to_string(),
            vault_id: vault_id.to_string(),
            target_type: TargetType::Item.value(),
            target_id: item_id.to_string(),
            owner: false,
            permission: 0,
            share_role_id: "1".to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 0,
            group_id: Some(group_id.to_string()),
        };
        let share_response_clone = share_response.clone();
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}", share_id),
            move |_| success(share_response_clone.clone()),
        );
        let share_response_clone2 = share_response.clone();
        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(crate::share::list::GetSharesResponse {
                shares: vec![share_response_clone2.clone()],
            })
        });
    }

    // Helper function to setup vault share with group
    fn setup_vault_share_with_group(server: &Arc<Server>, share_id: &str, group_id: &str) {
        let share_response = ShareResponse {
            share_id: share_id.to_string(),
            address_id: TEST_ADDRESS_ID.to_string(),
            vault_id: TEST_VAULT_ID.to_string(),
            target_type: TargetType::Vault.value(),
            target_id: TEST_VAULT_ID.to_string(),
            owner: true,
            permission: 0,
            share_role_id: "1".to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 0,
            group_id: Some(group_id.to_string()),
        };
        let share_response_clone = share_response.clone();
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}", share_id),
            move |_| success(share_response_clone.clone()),
        );
        let share_response_clone2 = share_response.clone();
        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(crate::share::list::GetSharesResponse {
                shares: vec![share_response_clone2.clone()],
            })
        });
    }

    // Helper function to setup group crypto infrastructure
    fn setup_group_crypto(
        server: &Arc<Server>,
        group_id: String,
        group_address_id: String,
        group_address_email: String,
        group_address_key_id: String,
        group_armored_public_key: String,
    ) {
        server.handler_with_method(Method::GET, "/core/v4/groups", move |_| {
            success(GetGroupsResponse {
                groups: vec![GroupResponse {
                    id: group_id.to_string(),
                    name: "Test group name".to_string(),
                    address: Some(addresses::Address {
                        id: group_address_id.to_string(),
                        email: group_address_email.to_string(),
                        keys: vec![keys::Key {
                            id: group_address_key_id.to_string(),
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
    }

    // Helper function to setup share keys for group shares
    fn setup_share_keys_for_group(
        server: &Arc<Server>,
        share_id: &str,
        encrypted_share_key: Vec<u8>,
    ) {
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/key", share_id),
            move |_| {
                success(GetShareKeysResponse {
                    keys: ShareKeyList {
                        keys: vec![ShareKeyResponse {
                            key_rotation: 1,
                            key: crate::utils::b64_encode(&encrypted_share_key),
                            create_time: 123456789,
                        }],
                        total: 1,
                    },
                })
            },
        );
    }

    // Helper function to create test item data
    fn create_test_item_data(title: &str, note: &str) -> ItemData {
        ItemData::new(
            title.to_string(),
            note.to_string(),
            random_string(10),
            ItemContent::Custom(CustomItem {
                sections: vec![CustomSection {
                    section_name: "Test Section".to_string(),
                    section_fields: vec![ItemExtraField {
                        name: "Test Field".to_string(),
                        content: ItemExtraFieldContent::Text("Test Value".to_string()),
                    }],
                }],
            }),
            vec![],
        )
        .expect("Error creating item data")
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_empty_list(server: Arc<Server>) {
        const SHARE_ID: &str = "EMPTY_SHARE_ID";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![])
            .await
            .expect("Should handle empty list");

        assert_eq!(0, result.len(), "Empty list should return empty result");
    }

    #[muon::test(scheme(HTTP))]
    async fn test_get_item_key_vault_share(server: Arc<Server>) {
        const SHARE_ID: &str = "VAULT_SHARE_ID";
        const ITEM_ID: &str = "VAULT_ITEM_ID";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Create a simple item revision for testing get_item_key
        let item_data = create_test_item_data("Test", "Note");
        let encrypted_data = encrypt_item_contents(item_data);

        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .build();

        let result = client
            .get_item_key(&share_id!(SHARE_ID), &item_revision)
            .await
            .expect("Should get item key");

        assert_eq!(1, result.key_rotation);
        assert_eq!(encrypted_data.item_key.as_ref(), result.key.as_ref())
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_vault_share_single_item(server: Arc<Server>) {
        const SHARE_ID: &str = "VAULT_SHARE_ID";
        const ITEM_ID: &str = "VAULT_ITEM_ID";
        const ITEM_TITLE: &str = "Test Vault Item";
        const ITEM_NOTE: &str = "Test vault item note";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Create test item data and encrypt it
        let item_data = create_test_item_data(ITEM_TITLE, ITEM_NOTE);
        let encrypted_data = encrypt_item_contents(item_data.clone());

        // Create item revision for vault share (has item_key)
        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_data.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .with_flags(0)
            .build();

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![item_revision])
            .await
            .expect("Should be able to open items");

        assert_eq!(1, result.len(), "Should return one item");
        let opened_item = &result[0];

        // Verify item properties
        assert_eq!(ITEM_ID, opened_item.item.id.value());
        assert_eq!(SHARE_ID, opened_item.item.share_id.value());
        assert_eq!(TEST_VAULT_ID, opened_item.item.vault_id.value());
        assert_eq!(ItemState::Active, opened_item.item.state);
        assert!(opened_item.item.flags.is_empty());

        // Verify decrypted content
        assert_eq!(ITEM_TITLE, opened_item.item.content.title);
        assert_eq!(ITEM_NOTE, opened_item.item.content.note);

        // Verify item key properties
        assert_eq!(1, opened_item.item_key.key_rotation);
        assert_eq!(32, opened_item.item_key.key.as_ref().len());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_item_share_single_item(server: Arc<Server>) {
        const SHARE_ID: &str = "ITEM_SHARE_ID";
        const ITEM_ID: &str = "SHARED_ITEM_ID";
        const VAULT_ID: &str = "ITEM_VAULT_ID";
        const ITEM_TITLE: &str = "Test Item Share";
        const ITEM_NOTE: &str = "Test item share note";

        let client = server.pass_client().await;
        setup_item_share(&server, SHARE_ID, ITEM_ID, VAULT_ID);
        setup_share_keys(&server, SHARE_ID);

        // Create test item data and encrypt it directly with share key (no item_key for item shares)
        let item_data = create_test_item_data(ITEM_TITLE, ITEM_NOTE);
        let serialized = item_data.serialize().expect("serialize data failed");
        let encrypted_content = crypto::encrypt(
            &serialized,
            &TEST_SHARE_KEY,
            crypto::EncryptionTag::ItemContent,
        )
        .expect("Error encrypting item content");

        // Create item revision for item share (no item_key)
        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_content))
            .with_item_key(None) // Item shares don't have separate item keys
            .with_state(ItemState::Active as u8)
            .build();

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![item_revision])
            .await
            .expect("Should open item share");

        assert_eq!(1, result.len(), "Should return one item");
        let opened_item = &result[0];

        // Verify item properties
        assert_eq!(ITEM_ID, opened_item.item.id.value());
        assert_eq!(SHARE_ID, opened_item.item.share_id.value());
        assert_eq!(VAULT_ID, opened_item.item.vault_id.value());
        assert_eq!(ItemState::Active, opened_item.item.state);

        // Verify decrypted content
        assert_eq!(ITEM_TITLE, opened_item.item.content.title);
        assert_eq!(ITEM_NOTE, opened_item.item.content.note);

        // Verify item key is the share key itself
        assert_eq!(1, opened_item.item_key.key_rotation);
        assert_eq!(TEST_SHARE_KEY.as_slice(), opened_item.item_key.key.as_ref());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_vault_share_with_group(server: Arc<Server>) {
        const SHARE_ID: &str = "VAULT_GROUP_SHARE_ID";
        const ITEM_ID: &str = "VAULT_GROUP_ITEM_ID";
        const GROUP_ID: &str = "VAULT_GROUP_ID";
        const GROUP_ADDRESS_ID: &str = "VAULT_GROUP_ADDRESS_ID";
        const GROUP_ADDRESS_EMAIL: &str = "vault-group@test.local";
        const GROUP_ADDRESS_KEY_ID: &str = "VAULT_GROUP_ADDRESS_KEY_ID";
        const ITEM_TITLE: &str = "Test Vault Group Item";
        const ITEM_NOTE: &str = "Test vault group item note";

        let client = server.pass_client().await;

        // Generate group key pair
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

        setup_vault_share_with_group(&server, SHARE_ID, GROUP_ID);
        setup_group_crypto(
            &server,
            GROUP_ID.to_string(),
            GROUP_ADDRESS_ID.to_string(),
            GROUP_ADDRESS_EMAIL.to_string(),
            GROUP_ADDRESS_KEY_ID.to_string(),
            group_armored_public_key,
        );

        // Create share key encrypted for group
        let share_key_raw = crypto::generate_encryption_key();
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

        setup_share_keys_for_group(&server, SHARE_ID, encrypted_share_key);

        // Create test item data and encrypt it
        let item_data = create_test_item_data(ITEM_TITLE, ITEM_NOTE);
        let serialized = item_data.serialize().expect("serialize data failed");
        let item_key = crypto::generate_encryption_key();
        let encrypted_content =
            crypto::encrypt(&serialized, &item_key, crypto::EncryptionTag::ItemContent)
                .expect("Error encrypting item content");
        let encrypted_item_key =
            crypto::encrypt(&item_key, &share_key_raw, crypto::EncryptionTag::ItemKey)
                .expect("Error encrypting item key");

        // Create item revision for vault share with group
        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_content))
            .with_item_key(Some(crate::utils::b64_encode(&encrypted_item_key)))
            .with_state(ItemState::Active as u8)
            .build();

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![item_revision])
            .await
            .expect("Should open vault share with group");

        assert_eq!(1, result.len(), "Should return one item");
        let opened_item = &result[0];

        // Verify item properties
        assert_eq!(ITEM_ID, opened_item.item.id.value());
        assert_eq!(SHARE_ID, opened_item.item.share_id.value());
        assert_eq!(TEST_VAULT_ID, opened_item.item.vault_id.value());
        assert_eq!(ItemState::Active, opened_item.item.state);

        // Verify decrypted content
        assert_eq!(ITEM_TITLE, opened_item.item.content.title);
        assert_eq!(ITEM_NOTE, opened_item.item.content.note);

        // Verify item key
        assert_eq!(1, opened_item.item_key.key_rotation);
        assert_eq!(item_key.as_slice(), opened_item.item_key.key.as_ref());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_item_share_with_group(server: Arc<Server>) {
        const SHARE_ID: &str = "ITEM_GROUP_SHARE_ID";
        const ITEM_ID: &str = "ITEM_GROUP_ITEM_ID";
        const VAULT_ID: &str = "ITEM_GROUP_VAULT_ID";
        const GROUP_ID: &str = "ITEM_GROUP_ID";
        const GROUP_ADDRESS_ID: &str = "ITEM_GROUP_ADDRESS_ID";
        const GROUP_ADDRESS_EMAIL: &str = "item-group@test.local";
        const GROUP_ADDRESS_KEY_ID: &str = "ITEM_GROUP_ADDRESS_KEY_ID";
        const ITEM_TITLE: &str = "Test Item Group Share";
        const ITEM_NOTE: &str = "Test item group share note";

        let client = server.pass_client().await;

        // Generate group key pair
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

        setup_item_share_with_group(&server, SHARE_ID, ITEM_ID, VAULT_ID, GROUP_ID);
        setup_group_crypto(
            &server,
            GROUP_ID.to_string(),
            GROUP_ADDRESS_ID.to_string(),
            GROUP_ADDRESS_EMAIL.to_string(),
            GROUP_ADDRESS_KEY_ID.to_string(),
            group_armored_public_key,
        );

        // Create share key encrypted for group (this IS the item key for item shares)
        let share_key_raw = crypto::generate_encryption_key();
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

        setup_share_keys_for_group(&server, SHARE_ID, encrypted_share_key);

        // Create test item data and encrypt it directly with share key
        let item_data = create_test_item_data(ITEM_TITLE, ITEM_NOTE);
        let serialized = item_data.serialize().expect("serialize data failed");
        let encrypted_content = crypto::encrypt(
            &serialized,
            &share_key_raw,
            crypto::EncryptionTag::ItemContent,
        )
        .expect("Error encrypting item content");

        // Create item revision for item share with group (no item_key)
        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_content))
            .with_item_key(None) // Item shares don't have separate item keys
            .with_state(ItemState::Active as u8)
            .build();

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![item_revision])
            .await
            .expect("Should open item share with group");

        assert_eq!(1, result.len(), "Should return one item");
        let opened_item = &result[0];

        // Verify item properties
        assert_eq!(ITEM_ID, opened_item.item.id.value());
        assert_eq!(SHARE_ID, opened_item.item.share_id.value());
        assert_eq!(VAULT_ID, opened_item.item.vault_id.value());
        assert_eq!(ItemState::Active, opened_item.item.state);

        // Verify decrypted content
        assert_eq!(ITEM_TITLE, opened_item.item.content.title);
        assert_eq!(ITEM_NOTE, opened_item.item.content.note);

        // Verify item key is the share key itself
        assert_eq!(1, opened_item.item_key.key_rotation);
        assert_eq!(share_key_raw.as_slice(), opened_item.item_key.key.as_ref());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_with_different_key_rotations(server: Arc<Server>) {
        const SHARE_ID: &str = "ROTATION_SHARE_ID";
        const ITEM_ID: &str = "ROTATION_ITEM_ID";
        const ITEM_TITLE: &str = "Test Rotation Item";
        const ITEM_NOTE: &str = "Test rotation item note";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);

        // Setup multiple share keys with different rotations
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/key", SHARE_ID),
            move |_| {
                success(GetShareKeysResponse {
                    keys: ShareKeyList {
                        keys: vec![
                            ShareKeyResponse {
                                key_rotation: 1,
                                key: TEST_SHARE_KEY_ENCRYPTED.to_string(),
                                create_time: 123456789,
                            },
                            ShareKeyResponse {
                                key_rotation: 2,
                                key: TEST_SHARE_KEY_ENCRYPTED.to_string(),
                                create_time: 123456790,
                            },
                        ],
                        total: 2,
                    },
                })
            },
        );

        // Create test item data and encrypt it
        let item_data = create_test_item_data(ITEM_TITLE, ITEM_NOTE);
        let encrypted_data = encrypt_item_contents(item_data.clone());

        // Create item revision with key_rotation = 2
        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_data.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .build();

        // Manually set key_rotation to 2
        let mut item_revision = item_revision;
        item_revision.key_rotation = 2;

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![item_revision])
            .await
            .expect("Should handle different key rotations");

        assert_eq!(1, result.len(), "Should return one item");
        let opened_item = &result[0];

        // Verify item key rotation
        assert_eq!(2, opened_item.item_key.key_rotation);
        assert_eq!(ITEM_TITLE, opened_item.item.content.title);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_open_items_with_item_flags_and_states(server: Arc<Server>) {
        const SHARE_ID: &str = "FLAGS_SHARE_ID";
        const ITEM_ID: &str = "FLAGS_ITEM_ID";
        const ITEM_TITLE: &str = "Test Flags Item";
        const ITEM_NOTE: &str = "Test flags item note";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Create test item data and encrypt it
        let item_data = create_test_item_data(ITEM_TITLE, ITEM_NOTE);
        let encrypted_data = encrypt_item_contents(item_data.clone());

        // Create item revision with specific flags and state
        let item_revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_data.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .with_state(ItemState::Trashed as u8)
            .with_flags(ItemFlag::SkipHealthCheck as u64)
            .build();

        let result = client
            .open_items(&share_id!(SHARE_ID), vec![item_revision])
            .await
            .expect("Should handle item flags and states");

        assert_eq!(1, result.len(), "Should return one item");
        let opened_item = &result[0];

        // Verify item state and flags
        assert_eq!(ItemState::Trashed, opened_item.item.state);
        assert_eq!(vec![ItemFlag::SkipHealthCheck], opened_item.item.flags);
        assert_eq!(ITEM_TITLE, opened_item.item.content.title);
    }
}
