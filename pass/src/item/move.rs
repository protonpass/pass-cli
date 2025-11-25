use crate::PassClient;
use crate::item::list::ItemRevision;
use crate::permission::PermissionAction;
use crate::utils::b64_encode;
use anyhow::{Context, Result};
use muon::PUT;
use pass_domain::{ItemId, ShareId, crypto};

#[derive(Debug, serde::Serialize)]
pub(crate) struct MoveItemRequest {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "Items")]
    pub items: Vec<MoveItemBody>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct MoveItemBody {
    #[serde(rename = "ItemID")]
    pub item_id: String,
    #[serde(rename = "ItemKeys")]
    pub item_keys: Vec<MoveItemKey>,
}

#[derive(Debug, serde::Serialize)]
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

impl PassClient {
    pub async fn move_item(
        &self,
        from_share_id: &ShareId,
        item_id: &ItemId,
        to_share_id: &ShareId,
    ) -> Result<ItemId> {
        self.action_guard(PermissionAction::DeleteItem {
            share_id: from_share_id.clone(),
            item_id: item_id.clone(),
        })
        .await?;
        self.action_guard(PermissionAction::CreateItem {
            share_id: to_share_id.clone(),
        })
        .await?;

        let body = self
            .create_move_item_request(from_share_id, item_id, to_share_id)
            .await
            .context("Error creating move item request body")?;
        let req = PUT!("/pass/v1/share/{from_share_id}/item/share")
            .body_json(body)
            .context("Error creating item move request")?;
        let res = self
            .send(req)
            .await
            .context("Error sending item move request")?;
        let response: MoveItemResponse = assert_response!(res);

        let new_item_id = response
            .items
            .first()
            .context("Error getting new item id")?
            .item_id
            .to_string();

        self.clear_items_cache(from_share_id).await;
        self.clear_items_cache(to_share_id).await;

        Ok(ItemId::new(new_item_id))
    }

    async fn create_move_item_request(
        &self,
        from_share_id: &ShareId,
        item_id: &ItemId,
        to_share_id: &ShareId,
    ) -> Result<MoveItemRequest> {
        // Get latest key rotation from API to ensure we use the most recent
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

        let source_item_keys = self
            .get_item_keys(from_share_id, item_id)
            .await
            .context("Error getting item keys")?;
        let opened_source_item_keys = self
            .open_item_keys(from_share_id, source_item_keys)
            .await
            .context("Error opening source item keys")?;

        let migrated_item_keys = {
            let mut migrated_item_keys = Vec::with_capacity(opened_source_item_keys.len());
            for item_key in opened_source_item_keys {
                let reencrypted_item_key = crypto::encrypt(
                    item_key.key.as_ref(),
                    opened_destination_share_key.as_ref(),
                    crypto::EncryptionTag::ItemKey,
                )
                .map_err(|e| anyhow::anyhow!("Error encrypting item key: {:?}", e))?;

                migrated_item_keys.push(MoveItemKey {
                    key_rotation: item_key.key_rotation,
                    key: b64_encode(&reencrypted_item_key),
                });
            }
            migrated_item_keys
        };

        let body = MoveItemRequest {
            share_id: to_share_id.to_string(),
            items: vec![MoveItemBody {
                item_id: item_id.to_string(),
                item_keys: migrated_item_keys,
            }],
        };

        Ok(body)
    }
}
