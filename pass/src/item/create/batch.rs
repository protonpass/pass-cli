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

use super::common::CreateItemRequest;
use crate::domain::{ItemData, ItemId, ShareId};
use crate::item::bulk::{
    BulkCreateResult, ItemOutcome, MAX_ITEMS_PER_CREATE_REQUEST, chunk_owned, fan_out_error,
    format_error, group_by_share, matched_ids,
};
use crate::item::list::ItemRevision;
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::POST;
use pass_derive::sdk_export;

#[derive(serde::Serialize)]
struct BatchImportItemBody {
    #[serde(rename = "Item")]
    item: CreateItemRequest,
    #[serde(rename = "AliasEmail")]
    alias_email: Option<String>,
    #[serde(rename = "Trashed")]
    trashed: bool,
    #[serde(rename = "CreateTime")]
    create_time: Option<u64>,
    #[serde(rename = "ModifyTime")]
    modify_time: Option<u64>,
}

#[derive(serde::Serialize)]
struct BatchCreateItemsRequest {
    #[serde(rename = "Items")]
    items: Vec<BatchImportItemBody>,
}

#[derive(serde::Deserialize)]
struct BatchCreateItemsResponse {
    #[serde(rename = "Revisions")]
    revisions: BatchItemRevisions,
}

#[derive(serde::Deserialize)]
struct BatchItemRevisions {
    #[serde(rename = "RevisionsData")]
    items: Vec<ItemRevision>,
}

fn build_batch_request(items: Vec<CreateItemRequest>) -> BatchCreateItemsRequest {
    BatchCreateItemsRequest {
        items: items
            .into_iter()
            .map(|item| BatchImportItemBody {
                item,
                alias_email: None,
                trashed: false,
                create_time: None,
                modify_time: None,
            })
            .collect(),
    }
}

#[sdk_export]
#[derive(Clone, Debug)]
pub struct NewItem {
    pub share_id: ShareId,
    pub data: ItemData,
}

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    #[sdk_export]
    pub async fn create_items_bulk(&self, items: Vec<NewItem>) -> Vec<BulkCreateResult> {
        let shares: Vec<ShareId> = items.iter().map(|item| item.share_id.clone()).collect();
        let outcomes = self.create_outcomes(items, false).await;
        shares
            .into_iter()
            .zip(outcomes)
            .map(|(share_id, outcome)| match outcome {
                Ok(item_id) => BulkCreateResult {
                    share_id,
                    item_id,
                    error: None,
                },
                Err(e) => BulkCreateResult {
                    share_id,
                    item_id: None,
                    error: Some(format_error(&e)),
                },
            })
            .collect()
    }

    pub async fn create_items(
        &self,
        share_id: &ShareId,
        items: Vec<ItemData>,
    ) -> Result<Vec<ItemId>> {
        let new_items = items
            .into_iter()
            .map(|data| NewItem {
                share_id: share_id.clone(),
                data,
            })
            .collect();
        let mut ids = Vec::new();
        for outcome in self.create_outcomes(new_items, true).await {
            ids.extend(outcome?);
        }
        Ok(ids)
    }

    async fn create_outcomes(&self, items: Vec<NewItem>, fail_fast: bool) -> Vec<ItemOutcome> {
        let mut results: Vec<ItemOutcome> = items.iter().map(|_| Ok(None)).collect();

        let indexed: Vec<(ShareId, (usize, ItemData))> = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| (item.share_id, (index, item.data)))
            .collect();

        let mut permitted = Vec::new();
        for (share_id, group) in group_by_share(indexed) {
            match self
                .action_guard(PermissionAction::CreateItem {
                    share_id: share_id.clone(),
                })
                .await
            {
                Ok(()) => permitted.push((share_id, group)),
                Err(e) => {
                    let indexes: Vec<usize> = group.iter().map(|(index, _)| *index).collect();
                    for (index, error) in indexes.iter().zip(fan_out_error(e, indexes.len())) {
                        results[*index] = Err(error);
                    }
                    if fail_fast {
                        return results;
                    }
                }
            }
        }

        let mut stopped = false;
        for (share_id, group) in permitted {
            if stopped {
                break;
            }
            let mut any_succeeded = false;
            for chunk in chunk_owned(group, MAX_ITEMS_PER_CREATE_REQUEST) {
                let mut requests = Vec::with_capacity(chunk.len());
                let mut indexes = Vec::with_capacity(chunk.len());
                for (index, data) in chunk {
                    match self
                        .create_item_request_from_data(&share_id, data, None)
                        .await
                        .context("Error creating item request")
                    {
                        Ok(request) => {
                            requests.push(request);
                            indexes.push(index);
                        }
                        Err(e) => {
                            results[index] = Err(e);
                            stopped = fail_fast;
                        }
                    }
                    if stopped {
                        break;
                    }
                }
                if stopped {
                    break;
                }
                if requests.is_empty() {
                    continue;
                }

                match self.send_batch_create(&share_id, requests).await {
                    Ok(ids) => {
                        any_succeeded = true;
                        for (index, id) in indexes.into_iter().zip(ids) {
                            results[index] = id.map(Some);
                        }
                    }
                    Err(e) => {
                        for (index, error) in indexes.iter().zip(fan_out_error(e, indexes.len())) {
                            results[*index] = Err(error);
                        }
                        stopped = fail_fast;
                    }
                }
                if stopped {
                    break;
                }
            }
            if any_succeeded {
                self.clear_items_cache(&share_id).await;
            }
        }

        results
    }

    async fn send_batch_create(
        &self,
        share_id: &ShareId,
        requests: Vec<CreateItemRequest>,
    ) -> Result<Vec<Result<ItemId>>> {
        let expected = requests.len();
        let req = POST!("/pass/v1/share/{share_id}/item/import/batch")
            .body_json(build_batch_request(requests))
            .context("Error serializing batch create items request")?;
        let response = self
            .send(req)
            .await
            .context("Error sending batch create items request")?;
        let response: BatchCreateItemsResponse = assert_response!(response);

        Ok(matched_ids(
            expected,
            response
                .revisions
                .items
                .into_iter()
                .map(|r| r.item_id)
                .collect(),
            "Error getting created item id",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(content: &str) -> CreateItemRequest {
        CreateItemRequest {
            key_rotation: 1,
            content_format_version: 7,
            content: content.to_string(),
            item_key: "key".to_string(),
            folder_id: None,
        }
    }

    #[test]
    fn build_batch_request_wraps_every_item() {
        let batch = build_batch_request(vec![request("a"), request("b")]);
        assert_eq!(batch.items.len(), 2);
        assert_eq!(batch.items[1].item.content, "b");
        assert!(!batch.items[0].trashed);
        assert!(batch.items[0].alias_email.is_none());
    }
}
