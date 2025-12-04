use crate::PassClient;
use crate::common::CodeResponse;
use crate::item::item_keys::OpenedItemKey;
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_domain::{FolderId, ItemId, ShareId, crypto};

#[derive(serde::Serialize)]
struct MoveItemKeyItem {
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
    #[serde(rename = "Key")]
    key: String,
}

#[derive(serde::Serialize)]
struct MoveItemToFolderItem {
    #[serde(rename = "ItemID")]
    item_id: String,
    #[serde(rename = "ItemKeys")]
    item_keys: Vec<MoveItemKeyItem>,
}

#[derive(serde::Serialize)]
struct MoveItemToFolderRequest {
    #[serde(rename = "FolderID")]
    folder_id: Option<String>,
    #[serde(rename = "Items")]
    items: Vec<MoveItemToFolderItem>,
}

impl PassClient {
    pub async fn move_item_to_folder(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        target_folder_id: Option<&FolderId>,
    ) -> Result<()> {
        // Get the item's current keys
        let item_keys = self
            .get_item_keys(share_id, item_id)
            .await
            .context("Error getting item keys")?;

        // Open all rotations of the item keys
        let opened_item_keys = self
            .open_item_keys(share_id, item_keys)
            .await
            .context("Error opening item keys")?;

        // Re-encrypt the item keys with the target folder key or share key
        let migrated_item_keys = if let Some(folder_id) = target_folder_id {
            // Moving to a folder
            let folder_rev = self
                .get_folder_data(share_id, folder_id)
                .await
                .context("Error getting target folder")?;

            let folder_key = self
                .get_opened_folder_key(share_id, folder_id, folder_rev.key_rotation)
                .await
                .context("Error opening target folder key")?;

            Self::reencrypt_item_keys(&opened_item_keys, folder_key.as_ref())?
        } else {
            // Moving to root (no folder)
            let share_keys = self
                .get_share_keys(share_id)
                .await
                .context("Error getting share keys")?;

            let share_key = share_keys.latest_or_err()?;
            let opened_share_key = self
                .get_opened_share_key_by_rotation(share_id, share_key.key_rotation)
                .await
                .context("Error opening share key")?;

            Self::reencrypt_item_keys(&opened_item_keys, opened_share_key.as_ref())?
        };

        // Build request
        let request = MoveItemToFolderRequest {
            folder_id: target_folder_id.map(|id| id.to_string()),
            items: vec![MoveItemToFolderItem {
                item_id: item_id.to_string(),
                item_keys: migrated_item_keys,
            }],
        };

        // Send request
        let req = PUT!("/pass/v1/share/{share_id}/item/folder")
            .body_json(request)
            .context("Error creating move item to folder request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending move item to folder request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        // Clear items cache
        self.clear_items_cache(share_id).await;

        Ok(())
    }

    fn reencrypt_item_keys(
        item_keys: &[OpenedItemKey],
        target_key: &[u8],
    ) -> Result<Vec<MoveItemKeyItem>> {
        let mut migrated_keys = Vec::with_capacity(item_keys.len());

        for item_key in item_keys {
            let reencrypted = crypto::encrypt(
                item_key.key.as_ref(),
                target_key,
                crypto::EncryptionTag::ItemKey,
            )
            .map_err(|e| {
                error!("Error encrypting item key: {e}");
                anyhow!("Error encrypting item key")
            })?;

            migrated_keys.push(MoveItemKeyItem {
                key_rotation: item_key.key_rotation,
                key: crate::utils::b64_encode(&reencrypted),
            });
        }

        Ok(migrated_keys)
    }
}
