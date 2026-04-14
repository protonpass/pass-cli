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

use crate::permission::PermissionAction;
use crate::utils::debug_response;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::DELETE;
use pass_domain::{ItemId, ItemType, ShareId, TelemetryEvent};
use std::collections::HashMap;

#[derive(Debug, serde::Serialize)]
struct DeleteItemsRequest {
    #[serde(rename = "Items")]
    items: Vec<DeleteItemBody>,
    #[serde(rename = "SkipTrash")]
    skip_trash: bool,
}

#[derive(Debug, serde::Serialize)]
struct DeleteItemBody {
    #[serde(rename = "ItemID")]
    item_id: String,
    #[serde(rename = "Revision")]
    revision: u64,
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

impl<C: PassClientContext> PassClient<C> {
    pub async fn delete_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<()> {
        self.action_guard(PermissionAction::DeleteItem {
            share_id: share_id.clone(),
            item_id: item_id.clone(),
        })
        .await?;
        let item_revision = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item")?;

        let item_type = self
            .get_item_type(share_id, &item_revision)
            .await
            .context("Error getting item type")?;

        let req = DELETE!("/pass/v1/share/{share_id}/item")
            .body_json(DeleteItemsRequest {
                items: vec![DeleteItemBody {
                    item_id: item_id.value().to_string(),
                    revision: item_revision.revision,
                }],
                skip_trash: true,
            })
            .context("Error creating delete item request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send delete item request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Error in delete item request: {}", res.status()));
        }

        // Clear the items cache for this share since we've deleted an item
        self.clear_items_cache(share_id).await;

        self.emit_telemetry(&ItemDeletedEvent { item_type }).await;

        Ok(())
    }
}
