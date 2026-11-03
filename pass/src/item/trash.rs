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
    BulkItemResult, ItemOutcome, ItemRef, MAX_ITEMS_PER_TRASH_REQUEST, RevisionedItem, chunk_owned,
    duplicate_positions, fan_out_error, group_by_share, resolve_duplicates,
};
use crate::common::CodeResponse;
use crate::domain::{ItemId, ItemState, ShareId};
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, bail};
use muon::POST;
use pass_derive::sdk_export;

#[derive(Debug, PartialEq, serde::Serialize)]
struct TrashItemsRequest {
    #[serde(rename = "Items")]
    items: Vec<TrashItemBody>,
}

#[derive(Debug, PartialEq, serde::Serialize)]
struct TrashItemBody {
    #[serde(rename = "ItemID")]
    item_id: String,
    #[serde(rename = "Revision")]
    revision: u64,
}

#[derive(Clone, Copy)]
enum TrashAction {
    Trash,
    Untrash,
}

impl TrashAction {
    fn expected_state(self) -> ItemState {
        match self {
            Self::Trash => ItemState::Active,
            Self::Untrash => ItemState::Trashed,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Trash => "trash",
            Self::Untrash => "untrash",
        }
    }
}

fn build_trash_request(items: &[RevisionedItem]) -> TrashItemsRequest {
    TrashItemsRequest {
        items: items
            .iter()
            .map(|item| TrashItemBody {
                item_id: item.item_id.value().to_string(),
                revision: item.revision,
            })
            .collect(),
    }
}

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    #[sdk_export]
    pub async fn trash_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<()> {
        self.trash_status_single(share_id, item_id, TrashAction::Trash)
            .await
    }

    #[sdk_export]
    pub async fn untrash_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<()> {
        self.trash_status_single(share_id, item_id, TrashAction::Untrash)
            .await
    }

    #[sdk_export]
    pub async fn trash_items(&self, items: Vec<ItemRef>) -> Vec<BulkItemResult> {
        self.trash_status_items(items, TrashAction::Trash).await
    }

    #[sdk_export]
    pub async fn untrash_items(&self, items: Vec<ItemRef>) -> Vec<BulkItemResult> {
        self.trash_status_items(items, TrashAction::Untrash).await
    }

    async fn trash_status_single(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        action: TrashAction,
    ) -> Result<()> {
        let item = ItemRef {
            share_id: share_id.clone(),
            item_id: item_id.clone(),
        };
        self.trash_status_outcomes(&[item], action)
            .await
            .into_iter()
            .next()
            .context("Missing result for item")?
            .map(|_| ())
    }

    async fn trash_status_items(
        &self,
        items: Vec<ItemRef>,
        action: TrashAction,
    ) -> Vec<BulkItemResult> {
        let outcomes = self.trash_status_outcomes(&items, action).await;
        BulkItemResult::from_outcomes(&items, outcomes)
    }

    async fn trash_status_outcomes(
        &self,
        items: &[ItemRef],
        action: TrashAction,
    ) -> Vec<ItemOutcome> {
        let mut results: Vec<ItemOutcome> = items.iter().map(|_| Ok(None)).collect();
        let duplicate_of = duplicate_positions(items);
        let permitted = self
            .guard_item_deletions(items, &duplicate_of, &mut results)
            .await;

        let mut prepared = Vec::with_capacity(permitted.len());
        for index in permitted {
            let item = &items[index];
            match self
                .prepare_trash_item(item, action)
                .await
                .with_context(|| format!("Error creating {} item request", action.label()))
            {
                Ok(revision) => prepared.push((
                    item.share_id.clone(),
                    RevisionedItem {
                        index,
                        item_id: item.item_id.clone(),
                        revision,
                    },
                )),
                Err(e) => results[index] = Err(e),
            }
        }

        for (share_id, group) in group_by_share(prepared) {
            let mut any_succeeded = false;
            for chunk in chunk_owned(group, MAX_ITEMS_PER_TRASH_REQUEST) {
                match self.send_trash_chunk(&share_id, &chunk, action).await {
                    Ok(()) => any_succeeded = true,
                    Err(e) => {
                        for (item, error) in chunk.iter().zip(fan_out_error(e, chunk.len())) {
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

    async fn send_trash_chunk(
        &self,
        share_id: &ShareId,
        chunk: &[RevisionedItem],
        action: TrashAction,
    ) -> Result<()> {
        let request = build_trash_request(chunk);
        let label = action.label();

        let req = match action {
            TrashAction::Trash => POST!("/pass/v1/share/{share_id}/item/trash"),
            TrashAction::Untrash => POST!("/pass/v1/share/{share_id}/item/untrash"),
        }
        .body_json(request)
        .with_context(|| format!("Error serializing {label} item request"))?;

        let res = self
            .send(req)
            .await
            .with_context(|| format!("Failed to send {label} item request"))?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;
        Ok(())
    }

    async fn prepare_trash_item(&self, item: &ItemRef, action: TrashAction) -> Result<u64> {
        let share_id = &item.share_id;
        let item_id = &item.item_id;
        let expected_state = action.expected_state();

        let item_revision = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item")?;

        let item_state: ItemState = item_revision
            .state
            .try_into()
            .context("Invalid item state")?;

        if item_state != expected_state {
            bail!(
                "Item must be in {expected_state:?} state to be trashed. Current state: {item_state:?}"
            );
        }

        Ok(item_revision.revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_trash_request_keeps_order_and_revisions() {
        let items = vec![
            RevisionedItem {
                index: 3,
                item_id: ItemId::new("a".to_string()),
                revision: 2,
            },
            RevisionedItem {
                index: 0,
                item_id: ItemId::new("b".to_string()),
                revision: 7,
            },
        ];
        let request = build_trash_request(&items);
        assert_eq!(
            request,
            TrashItemsRequest {
                items: vec![
                    TrashItemBody {
                        item_id: "a".to_string(),
                        revision: 2
                    },
                    TrashItemBody {
                        item_id: "b".to_string(),
                        revision: 7
                    },
                ]
            }
        );
        let json = serde_json::to_value(&request).unwrap_or_default();
        assert_eq!(json["Items"][1]["ItemID"], "b");
    }
}
