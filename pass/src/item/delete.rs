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

use super::bulk::{
    BulkItemResult, ItemOutcome, ItemRef, MAX_ITEMS_PER_DELETE_REQUEST, RevisionedItem,
    chunk_owned, duplicate_positions, fan_out_error, group_by_share, resolve_duplicates,
};
use crate::domain::{ItemId, ItemType, ShareId, TelemetryEvent};
use crate::utils::debug_response;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::DELETE;
use pass_derive::sdk_export;
use std::collections::HashMap;

#[derive(Debug, PartialEq, serde::Serialize)]
struct DeleteItemsRequest {
    #[serde(rename = "Items")]
    items: Vec<DeleteItemBody>,
    #[serde(rename = "SkipTrash")]
    skip_trash: bool,
}

#[derive(Debug, PartialEq, serde::Serialize)]
struct DeleteItemBody {
    #[serde(rename = "ItemID")]
    item_id: String,
    #[serde(rename = "Revision")]
    revision: u64,
}

fn build_delete_request(items: &[RevisionedItem]) -> DeleteItemsRequest {
    DeleteItemsRequest {
        items: items
            .iter()
            .map(|item| DeleteItemBody {
                item_id: item.item_id.value().to_string(),
                revision: item.revision,
            })
            .collect(),
        skip_trash: true,
    }
}

#[derive(Clone, Debug)]
pub struct ItemDeletedEvent {
    pub item_type: ItemType,
}

impl TelemetryEvent for ItemDeletedEvent {
    fn event_type(&self) -> String {
        "item.deletion".to_string()
    }

    fn dimensions(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("itemType".to_string(), self.item_type.as_str().to_string());
        map
    }
}

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    #[sdk_export]
    pub async fn delete_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<()> {
        let item = ItemRef {
            share_id: share_id.clone(),
            item_id: item_id.clone(),
        };
        self.delete_outcomes(&[item])
            .await
            .into_iter()
            .next()
            .context("Missing result for item")?
            .map(|_| ())
    }

    #[sdk_export]
    pub async fn delete_items(&self, items: Vec<ItemRef>) -> Vec<BulkItemResult> {
        let outcomes = self.delete_outcomes(&items).await;
        BulkItemResult::from_outcomes(&items, outcomes)
    }

    async fn delete_outcomes(&self, items: &[ItemRef]) -> Vec<ItemOutcome> {
        let mut results: Vec<ItemOutcome> = items.iter().map(|_| Ok(None)).collect();
        let duplicate_of = duplicate_positions(items);
        let permitted = self
            .guard_item_deletions(items, &duplicate_of, &mut results)
            .await;

        let mut prepared = Vec::with_capacity(permitted.len());
        for index in permitted {
            let item = &items[index];
            match self.prepare_delete_item(item).await {
                Ok((revision, item_type)) => prepared.push((
                    item.share_id.clone(),
                    (
                        RevisionedItem {
                            index,
                            item_id: item.item_id.clone(),
                            revision,
                        },
                        item_type,
                    ),
                )),
                Err(e) => results[index] = Err(e),
            }
        }

        for (share_id, group) in group_by_share(prepared) {
            let mut any_succeeded = false;
            for chunk in chunk_owned(group, MAX_ITEMS_PER_DELETE_REQUEST) {
                let revisions: Vec<RevisionedItem> =
                    chunk.iter().map(|(item, _)| item.clone()).collect();
                match self.send_delete_chunk(&share_id, &revisions).await {
                    Ok(()) => {
                        any_succeeded = true;
                        for (_, item_type) in chunk {
                            self.emit_telemetry(&ItemDeletedEvent { item_type }).await;
                        }
                    }
                    Err(e) => {
                        for ((item, _), error) in chunk.iter().zip(fan_out_error(e, chunk.len())) {
                            results[item.index] = Err(error);
                        }
                    }
                }
            }
            if any_succeeded {
                self.clear_items_cache(&share_id).await;
            }
        }

        resolve_duplicates(&mut results, &duplicate_of);
        results
    }

    async fn prepare_delete_item(&self, item: &ItemRef) -> Result<(u64, ItemType)> {
        let item_revision = self
            .fetch_item_revision(&item.share_id, &item.item_id)
            .await
            .context("Error fetching item")?;

        let item_type = self
            .get_item_type(&item.share_id, &item_revision)
            .await
            .context("Error getting item type")?;

        Ok((item_revision.revision, item_type))
    }

    async fn send_delete_chunk(&self, share_id: &ShareId, chunk: &[RevisionedItem]) -> Result<()> {
        let req = DELETE!("/pass/v1/share/{share_id}/item")
            .body_json(build_delete_request(chunk))
            .context("Error creating delete item request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send delete item request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Error in delete item request: {}", res.status()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_delete_request_skips_trash() {
        let items = vec![RevisionedItem {
            index: 0,
            item_id: ItemId::new("a".to_string()),
            revision: 4,
        }];
        assert_eq!(
            build_delete_request(&items),
            DeleteItemsRequest {
                items: vec![DeleteItemBody {
                    item_id: "a".to_string(),
                    revision: 4
                }],
                skip_trash: true,
            }
        );
    }
}
