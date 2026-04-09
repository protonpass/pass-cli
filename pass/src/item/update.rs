use crate::common::CodeResponse;
use crate::constants::ITEM_CONTENT_CONTENT_FORMAT_VERSION;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_domain::{ItemData, ItemId, ItemType, ShareId, TelemetryEvent, crypto};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(crate) struct ItemUpdatedEvent {
    pub item_type: ItemType,
}

impl TelemetryEvent for ItemUpdatedEvent {
    fn event_type(&self) -> String {
        "item.update".to_string()
    }

    fn dimensions(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("itemType".to_string(), self.item_type.as_str().to_string());
        map
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct UpdateItemRequest {
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
    #[serde(rename = "LastRevision")]
    last_revision: u64,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "ContentFormatVersion")]
    content_format_version: u32,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn update_item(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        new_content: ItemData,
    ) -> Result<()> {
        let item_revision = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item revision")?;

        let item_type = ItemType::from_content(&new_content.content);

        let item_key = self
            .get_item_key(share_id, &item_revision)
            .await
            .context("Error getting item key")?;

        let decoded_revision_content = crate::utils::b64_decode(&item_revision.content)
            .context("Error decoding original item content")?;

        let original_decrypted = crypto::decrypt(
            &decoded_revision_content,
            item_key.key.as_ref(),
            crypto::EncryptionTag::ItemContent,
        )
        .map_err(|e| {
            error!("Error decrypting original revision content: {e:#}");
            anyhow!("Error decrypting original revision content")
        })?;

        let updated_content = ItemData::perform_update(&original_decrypted, &new_content)
            .context("Error updating item contents")?;

        let encrypted_content = crypto::encrypt(
            &updated_content,
            item_key.key.as_ref(),
            crypto::EncryptionTag::ItemContent,
        )
        .map_err(|e| {
            error!("Error encrypting item content: {e:#}");
            anyhow!("Error encrypting item content")
        })?;

        let request = UpdateItemRequest {
            key_rotation: item_revision.key_rotation,
            last_revision: item_revision.revision,
            content: crate::utils::b64_encode(&encrypted_content),
            content_format_version: ITEM_CONTENT_CONTENT_FORMAT_VERSION,
        };

        let req = PUT!("/pass/v1/share/{share_id}/item/{item_id}")
            .body_json(request)
            .context("Error creating update item request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send update item request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_items_cache(share_id).await;
        self.emit_telemetry(&ItemUpdatedEvent { item_type }).await;

        Ok(())
    }
}
