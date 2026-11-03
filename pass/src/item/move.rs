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
    BulkItemResult, ItemOutcome, ItemRef, MAX_ITEMS_PER_MOVE_REQUEST, chunk_owned,
    duplicate_positions, fan_out_error, group_by_share, matched_ids, resolve_duplicates,
};
use crate::domain::{DecryptedShareKey, ItemId, ShareId, crypto};
use crate::item::list::ItemRevision;
use crate::permission::PermissionAction;
use crate::utils::b64_encode;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_derive::sdk_export;

#[derive(Debug, PartialEq, serde::Serialize)]
pub(crate) struct MoveItemRequest {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "Items")]
    pub items: Vec<MoveItemBody>,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub(crate) struct MoveItemBody {
    #[serde(rename = "ItemID")]
    pub item_id: String,
    #[serde(rename = "ItemKeys")]
    pub item_keys: Vec<MoveItemKey>,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub(crate) struct MoveItemKey {
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "Key")]
    pub key: String,
}

#[derive(Debug, serde::Deserialize)]
struct MoveItemResponse {
    #[serde(rename = "Items")]
    items: Vec<ItemRevision>,
}

fn build_move_request(to_share_id: &ShareId, items: Vec<MoveItemBody>) -> MoveItemRequest {
    MoveItemRequest {
        share_id: to_share_id.to_string(),
        items,
    }
}

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    #[sdk_export]
    pub async fn move_item(
        &self,
        from_share_id: &ShareId,
        item_id: &ItemId,
        to_share_id: &ShareId,
    ) -> Result<ItemId> {
        let item = ItemRef {
            share_id: from_share_id.clone(),
            item_id: item_id.clone(),
        };
        self.move_outcomes(&[item], to_share_id)
            .await?
            .into_iter()
            .next()
            .context("Missing result for item")??
            .context("Error getting new item id")
    }

    #[sdk_export]
    pub async fn move_items(
        &self,
        items: Vec<ItemRef>,
        to_share_id: &ShareId,
    ) -> Vec<BulkItemResult> {
        let outcomes = match self.move_outcomes(&items, to_share_id).await {
            Ok(outcomes) => outcomes,
            Err(e) => fan_out_error(e, items.len()).into_iter().map(Err).collect(),
        };
        BulkItemResult::from_outcomes(&items, outcomes)
    }

    async fn move_outcomes(
        &self,
        items: &[ItemRef],
        to_share_id: &ShareId,
    ) -> Result<Vec<ItemOutcome>> {
        let mut results: Vec<ItemOutcome> = items.iter().map(|_| Ok(None)).collect();
        if items.is_empty() {
            return Ok(results);
        }
        let duplicate_of = duplicate_positions(items);
        let permitted = self
            .guard_item_deletions(items, &duplicate_of, &mut results)
            .await;
        self.action_guard(PermissionAction::CreateItem {
            share_id: to_share_id.clone(),
        })
        .await?;

        if permitted.is_empty() {
            resolve_duplicates(&mut results, &duplicate_of);
            return Ok(results);
        }

        let destination_key = self
            .destination_share_key(to_share_id)
            .await
            .context("Error creating move item request body")?;

        let mut any_succeeded = false;
        let mut affected_sources: Vec<ShareId> = Vec::new();
        let indexed: Vec<(ShareId, (usize, ItemId))> = permitted
            .into_iter()
            .map(|index| {
                let item = &items[index];
                (item.share_id.clone(), (index, item.item_id.clone()))
            })
            .collect();
        for (from_share_id, group) in group_by_share(indexed) {
            for chunk in chunk_owned(group, MAX_ITEMS_PER_MOVE_REQUEST) {
                let mut bodies = Vec::with_capacity(chunk.len());
                let mut indexes = Vec::with_capacity(chunk.len());
                for (index, item_id) in chunk {
                    match self
                        .create_move_item_body(&from_share_id, &item_id, &destination_key)
                        .await
                        .context("Error creating move item request body")
                    {
                        Ok(body) => {
                            bodies.push(body);
                            indexes.push(index);
                        }
                        Err(e) => results[index] = Err(e),
                    }
                }
                if bodies.is_empty() {
                    continue;
                }

                match self
                    .send_move_chunk(&from_share_id, to_share_id, bodies)
                    .await
                {
                    Ok(ids) => {
                        any_succeeded = true;
                        if !affected_sources.contains(&from_share_id) {
                            affected_sources.push(from_share_id.clone());
                        }
                        for (index, id) in indexes.into_iter().zip(ids) {
                            results[index] = id.map(Some);
                        }
                    }
                    Err(e) => {
                        for (index, error) in indexes.iter().zip(fan_out_error(e, indexes.len())) {
                            results[*index] = Err(error);
                        }
                    }
                }
            }
        }

        for share_id in &affected_sources {
            self.clear_items_cache(share_id).await;
        }
        if any_succeeded {
            self.clear_items_cache(to_share_id).await;
        }

        resolve_duplicates(&mut results, &duplicate_of);
        Ok(results)
    }

    async fn destination_share_key(&self, to_share_id: &ShareId) -> Result<DecryptedShareKey> {
        let destination_share_keys = self
            .get_share_keys(to_share_id)
            .await
            .context("Error getting destination share keys")?;
        let destination_share_key = destination_share_keys
            .latest()
            .context("Error getting destination share key")?;

        let opened_destination_share_key = self
            .get_opened_share_key_by_rotation(to_share_id, destination_share_key.key_rotation)
            .await
            .context("Error opening destination share key")?;

        Ok(opened_destination_share_key)
    }

    async fn send_move_chunk(
        &self,
        from_share_id: &ShareId,
        to_share_id: &ShareId,
        bodies: Vec<MoveItemBody>,
    ) -> Result<Vec<Result<ItemId>>> {
        let expected = bodies.len();
        let req = PUT!("/pass/v1/share/{from_share_id}/item/share")
            .body_json(build_move_request(to_share_id, bodies))
            .context("Error creating item move request")?;
        let res = self
            .send(req)
            .await
            .context("Error sending item move request")?;
        let response: MoveItemResponse = assert_response!(res);

        // The backend returns moved items in the same order as the request, so new IDs are matched by position.
        Ok(matched_ids(
            expected,
            response.items.into_iter().map(|i| i.item_id).collect(),
            "Error getting new item id",
        ))
    }

    async fn create_move_item_body(
        &self,
        from_share_id: &ShareId,
        item_id: &ItemId,
        destination_share_key: &DecryptedShareKey,
    ) -> Result<MoveItemBody> {
        let source_item_keys = self
            .get_item_keys(from_share_id, item_id)
            .await
            .context("Error getting item keys")?;
        let opened_source_item_keys = self
            .open_item_keys(from_share_id, source_item_keys)
            .await
            .context("Error opening source item keys")?;

        let mut migrated_item_keys = Vec::with_capacity(opened_source_item_keys.len());
        for item_key in opened_source_item_keys {
            let reencrypted_item_key = crypto::encrypt(
                item_key.key.as_ref(),
                destination_share_key.as_ref(),
                crypto::EncryptionTag::ItemKey,
            )
            .map_err(|e| anyhow!("Error encrypting item key: {:?}", e))?;

            migrated_item_keys.push(MoveItemKey {
                key_rotation: item_key.key_rotation,
                key: b64_encode(&reencrypted_item_key),
            });
        }

        Ok(MoveItemBody {
            item_id: item_id.to_string(),
            item_keys: migrated_item_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_move_request_targets_destination() {
        let request = build_move_request(
            &ShareId::new("dest".to_string()),
            vec![MoveItemBody {
                item_id: "a".to_string(),
                item_keys: vec![MoveItemKey {
                    key_rotation: 1,
                    key: "k".to_string(),
                }],
            }],
        );
        assert_eq!(request.share_id, "dest");
        assert_eq!(request.items.len(), 1);
        assert_eq!(request.items[0].item_keys[0].key_rotation, 1);
    }
}
