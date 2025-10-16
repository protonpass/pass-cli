use crate::PassClient;
use crate::permission::PermissionAction;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::DELETE;
use pass_domain::{ItemId, ShareId};

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

impl PassClient {
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
        let req = DELETE!(
            "/pass/v1/share/{share_id}/item/{item_id}",
            share_id = share_id,
            item_id = item_id
        )
        .body_json(DeleteItemsRequest {
            items: vec![DeleteItemBody {
                item_id: item_id.value().to_string(),
                revision: item_revision.revision,
            }],
            skip_trash: false,
        })
        .context("Error creating delete item request")?;

        let res = self
            .client
            .send(req)
            .await
            .context("Failed to send delete item request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Error in delete item request: {}", res.status()));
        }

        // Clear the items cache for this share since we've deleted an item
        self.clear_items_cache(share_id).await;
        Ok(())
    }
}
