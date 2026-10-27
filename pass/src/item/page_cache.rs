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

use crate::domain::ShareId;
use crate::item::open::ItemWithItemKey;
use crate::pagination::SincePagination;
use anyhow::{Context, Result};
use async_lock::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct ItemsForShareCacheType;

pub(crate) type ShareItemsHandle = Arc<Mutex<ShareItems>>;
pub(crate) type ItemsForShareCache = HashMap<ShareId, ShareItemsHandle>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FillState {
    Partial(SincePagination),
    Complete(SincePagination),
}

impl FillState {
    pub(crate) fn cursor(&self) -> &SincePagination {
        match self {
            Self::Partial(cursor) | Self::Complete(cursor) => cursor,
        }
    }
}

impl Default for FillState {
    fn default() -> Self {
        Self::Partial(SincePagination::default())
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct SerializedItemsWithKey(Vec<u8>);

impl SerializedItemsWithKey {
    pub fn new(content: &[ItemWithItemKey], xor_key: u8) -> Result<Self> {
        let serialized = serde_json::to_vec(content).context("Error serializing content")?;
        Ok(Self(Self::perform_xor(serialized, xor_key)))
    }

    pub fn deserialize(&self, xor_key: u8) -> Result<Vec<ItemWithItemKey>> {
        if self.0.is_empty() {
            return Ok(Vec::new());
        }
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

pub(crate) struct ShareItems {
    xor_key: u8,
    pages: Vec<SerializedItemsWithKey>,
    state: FillState,
}

impl ShareItems {
    pub(crate) fn new(xor_key: u8) -> Self {
        Self {
            xor_key,
            pages: Vec::new(),
            state: FillState::default(),
        }
    }

    pub(crate) fn state(&self) -> &FillState {
        &self.state
    }

    pub(crate) fn set_state(&mut self, state: FillState) {
        self.state = state;
    }

    pub(crate) fn items(&mut self) -> Vec<ItemWithItemKey> {
        let mut items = Vec::new();
        for page in &self.pages {
            match page.deserialize(self.xor_key) {
                Ok(page_items) => items.extend(page_items),
                Err(e) => {
                    warn!("Error deserializing cached items, resetting cache: {e:#}");
                    self.pages.clear();
                    self.state = FillState::default();
                    return Vec::new();
                }
            }
        }
        items
    }

    pub(crate) fn append(&mut self, page: &[ItemWithItemKey], state: FillState) -> Result<()> {
        if !page.is_empty() {
            let serialized =
                SerializedItemsWithKey::new(page, self.xor_key).context("Error caching items")?;
            self.pages.push(serialized);
        }
        self.state = state;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Item, ItemState};
    use crate::item::item_keys::OpenedItemKey;
    use crate::test_tools::{create_random_item, encrypt_item_contents};

    const XOR_KEY: u8 = 0x42;

    fn item_with_key(id: &str) -> ItemWithItemKey {
        let content = create_random_item();
        let encrypted = encrypt_item_contents(content.clone());
        ItemWithItemKey {
            item: Item {
                id: item_id!(id),
                share_id: share_id!("test_share_id"),
                vault_id: vault_id!("test_vault_id"),
                content,
                state: ItemState::Active,
                flags: vec![],
                alias_email: None,
                create_time: jiff::Timestamp::from_second(1234567890)
                    .expect("valid timestamp")
                    .to_zoned(jiff::tz::TimeZone::UTC)
                    .datetime(),
                modify_time: jiff::Timestamp::from_second(1234567890)
                    .expect("valid timestamp")
                    .to_zoned(jiff::tz::TimeZone::UTC)
                    .datetime(),
                folder_id: None,
            },
            item_key: OpenedItemKey {
                key_rotation: 1,
                key: encrypted.item_key,
            },
        }
    }

    fn ids(items: &[ItemWithItemKey]) -> Vec<String> {
        items
            .iter()
            .map(|i| i.item.id.value().to_string())
            .collect()
    }

    #[test]
    fn serialized_items_round_trip() {
        let items = vec![item_with_key("a"), item_with_key("b")];
        let serialized = SerializedItemsWithKey::new(&items, XOR_KEY).expect("should serialize");
        let deserialized = serialized.deserialize(XOR_KEY).expect("should deserialize");

        assert_eq!(ids(&items), ids(&deserialized));
        assert_eq!(
            items[0].item_key.key_rotation,
            deserialized[0].item_key.key_rotation
        );
    }

    #[test]
    fn serialized_items_wrong_key_fails() {
        let items = vec![item_with_key("a")];
        let serialized = SerializedItemsWithKey::new(&items, XOR_KEY).expect("should serialize");
        assert!(serialized.deserialize(0x99).is_err());
    }

    #[test]
    fn serialized_items_empty_round_trip() {
        let serialized =
            SerializedItemsWithKey::new(&[], XOR_KEY).expect("should serialize empty list");
        assert!(
            serialized
                .deserialize(XOR_KEY)
                .expect("should deserialize")
                .is_empty()
        );
    }

    #[test]
    fn serialized_items_xor_actually_transforms_bytes() {
        let items = vec![item_with_key("a")];
        let plain = serde_json::to_vec(&items).expect("should serialize to json");
        let serialized = SerializedItemsWithKey::new(&items, XOR_KEY).expect("should serialize");

        assert_ne!(plain, serialized.0);
        for (original, transformed) in plain.iter().zip(serialized.0.iter()) {
            assert_eq!(original ^ XOR_KEY, *transformed);
        }
    }

    #[test]
    fn share_items_starts_partial_from_first_page() {
        let mut cache = ShareItems::new(XOR_KEY);
        assert_eq!(
            &FillState::Partial(SincePagination::default()),
            cache.state()
        );
        assert!(cache.items().is_empty());
    }

    #[test]
    fn share_items_append_accumulates_and_moves_cursor() {
        let mut cache = ShareItems::new(XOR_KEY);
        let first = vec![item_with_key("a"), item_with_key("b")];
        let next = FillState::Partial(SincePagination {
            since: Some("token".to_string()),
            page_size: 10,
        });

        cache.append(&first, next.clone()).expect("should append");
        assert_eq!(ids(&first), ids(&cache.items()));
        assert_eq!(&next, cache.state());

        let second = vec![item_with_key("c")];
        cache
            .append(&second, FillState::Complete(SincePagination::default()))
            .expect("should append");
        assert_eq!(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ids(&cache.items())
        );
        assert_eq!(
            &FillState::Complete(SincePagination::default()),
            cache.state()
        );
    }

    #[test]
    fn share_items_set_state_keeps_items() {
        let mut cache = ShareItems::new(XOR_KEY);
        let page = vec![item_with_key("a")];
        cache
            .append(&page, FillState::Partial(SincePagination::default()))
            .expect("should append");

        let complete = FillState::Complete(SincePagination::default());
        cache.set_state(complete.clone());
        assert_eq!(&complete, cache.state());
        assert_eq!(ids(&page), ids(&cache.items()));
    }

    #[test]
    fn share_items_corrupt_page_resets_cache() {
        let mut cache = ShareItems::new(XOR_KEY);
        cache
            .append(
                &[item_with_key("a")],
                FillState::Partial(SincePagination::default()),
            )
            .expect("should append");
        cache
            .append(
                &[item_with_key("b")],
                FillState::Complete(SincePagination {
                    since: Some("token".to_string()),
                    page_size: 10,
                }),
            )
            .expect("should append");
        cache.pages[1] = SerializedItemsWithKey(b"not json".to_vec());

        assert!(cache.items().is_empty());
        assert_eq!(&FillState::default(), cache.state());
        assert!(cache.pages.is_empty());
    }

    #[test]
    fn fill_state_cursor_is_kept_when_complete() {
        let cursor = SincePagination {
            since: Some("token".to_string()),
            page_size: 10,
        };
        assert_eq!(&cursor, FillState::Partial(cursor.clone()).cursor());
        assert_eq!(&cursor, FillState::Complete(cursor.clone()).cursor());
    }
}
