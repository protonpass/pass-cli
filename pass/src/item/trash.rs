use crate::common::CodeResponse;
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, bail};
use muon::POST;
use pass_domain::{ItemId, ItemState, ShareId};

#[derive(Debug, serde::Serialize)]
struct TrashItemsRequest {
    #[serde(rename = "Items")]
    items: Vec<TrashItemBody>,
}

#[derive(Debug, serde::Serialize)]
struct TrashItemBody {
    #[serde(rename = "ItemID")]
    item_id: String,
    #[serde(rename = "Revision")]
    revision: u64,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn trash_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<()> {
        let request = self
            .trash_status_item_request(share_id, item_id, ItemState::Active)
            .await
            .context("Error creating trash item request")?;

        let req = POST!("/pass/v1/share/{share_id}/item/trash")
            .body_json(request)
            .context("Error serializing trash item request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send trash item request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_items_cache(share_id).await;
        Ok(())
    }

    pub async fn untrash_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<()> {
        let request = self
            .trash_status_item_request(share_id, item_id, ItemState::Trashed)
            .await
            .context("Error creating untrash item request")?;

        let req = POST!("/pass/v1/share/{share_id}/item/untrash")
            .body_json(request)
            .context("Error serializing untrash item request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send untrash item request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_items_cache(share_id).await;
        Ok(())
    }

    async fn trash_status_item_request(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        expected_state: ItemState,
    ) -> Result<TrashItemsRequest> {
        self.action_guard(PermissionAction::DeleteItem {
            share_id: share_id.clone(),
            item_id: item_id.clone(),
        })
        .await?;

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

        Ok(TrashItemsRequest {
            items: vec![TrashItemBody {
                item_id: item_id.value().to_string(),
                revision: item_revision.revision,
            }],
        })
    }
}
