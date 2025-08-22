use crate::PassClient;
use crate::constants::ITEM_CONTENT_CONTENT_FORMAT_VERSION;
use anyhow::{Context, Result, anyhow};
use pass_domain::{ItemContent, ItemData, ShareId, crypto};

#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct CreateItemRequest {
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: u32,
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ItemKey")]
    pub item_key: String,
}

impl PassClient {
    pub(crate) async fn create_item_request(
        &self,
        share_id: &ShareId,
        title: &str,
        item_content: ItemContent,
    ) -> Result<CreateItemRequest> {
        let share_keys = self
            .get_share_keys(share_id)
            .await
            .context("Error retrieving share keys")?;

        let share_key = share_keys.latest_or_err()?;

        let item_key = crypto::generate_encryption_key();

        let content = ItemData {
            title: title.to_string(),
            note: "".to_string(),
            item_uuid: ItemData::generate_uuid(),
            content: item_content,
            extra_fields: vec![],
        };
        let serialized_content = content
            .serialize()
            .context("Error serializing item content")?;
        let encrypted_item_content = crypto::encrypt(
            &serialized_content,
            &item_key,
            crypto::EncryptionTag::ItemContent,
        )
        .map_err(|e| {
            error!("Error encrypting item contents: {e}");
            anyhow!("Error encrypting item contents")
        })?;

        let opened_share_key = self
            .open_share_key_for_share_id(share_id, share_key.clone())
            .await
            .context("Error opening share key")?;

        let encrypted_item_key = crypto::encrypt(
            &item_key,
            opened_share_key.as_ref(),
            crypto::EncryptionTag::ItemKey,
        )
        .map_err(|e| {
            error!("Error encrypting item key: {e}");
            anyhow!("Error encrypting item key")
        })?;

        Ok(CreateItemRequest {
            key_rotation: share_key.key_rotation,
            content_format_version: ITEM_CONTENT_CONTENT_FORMAT_VERSION,
            content: crate::utils::b64_encode(encrypted_item_content),
            item_key: crate::utils::b64_encode(encrypted_item_key),
        })
    }
}
