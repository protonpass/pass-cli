use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_domain::{FolderId, ShareId, crypto};

#[derive(serde::Serialize)]
struct MoveFolderKeyItem {
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
    #[serde(rename = "FolderKey")]
    folder_key: String,
}

#[derive(serde::Serialize)]
struct MoveFolderRequest {
    #[serde(rename = "ParentFolderID")]
    parent_folder_id: Option<String>,
    #[serde(rename = "FolderKeys")]
    folder_keys: Vec<MoveFolderKeyItem>,
}

impl PassClient {
    pub async fn move_folder(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
        new_parent_id: Option<FolderId>,
    ) -> Result<()> {
        // Get the folder revision
        let folder_rev = self
            .get_folder_data(share_id, folder_id)
            .await
            .context("Error getting folder revision")?;

        // Open the folder's current key (decrypt it)
        let folder_key = self
            .get_opened_folder_key(share_id, folder_id, folder_rev.key_rotation)
            .await
            .context("Error opening folder key")?;

        // Re-encrypt the folder key with the new parent's key or share key
        let (encrypted_folder_key, key_rotation) = if let Some(ref parent_id) = new_parent_id {
            // Moving to another folder
            let parent_rev = self
                .get_folder_data(share_id, parent_id)
                .await
                .context("Error getting new parent folder")?;

            let parent_key = self
                .get_opened_folder_key(share_id, parent_id, parent_rev.key_rotation)
                .await
                .context("Error opening new parent folder key")?;

            let encrypted = crypto::encrypt(
                folder_key.as_ref(),
                parent_key.as_ref(),
                crypto::EncryptionTag::FolderKey,
            )
            .map_err(|e| {
                error!("Error encrypting folder key with new parent key: {e}");
                anyhow!("Error encrypting folder key with new parent key")
            })?;

            (encrypted, parent_rev.key_rotation)
        } else {
            // Moving to root
            let share_keys = self
                .get_share_keys(share_id)
                .await
                .context("Error getting share keys")?;

            let share_key = share_keys.latest_or_err()?;
            let key_rotation = share_key.key_rotation;

            let opened_share_key = self
                .get_opened_share_key_by_rotation(share_id, key_rotation)
                .await
                .context("Error opening share key")?;

            let encrypted = crypto::encrypt(
                folder_key.as_ref(),
                opened_share_key.as_ref(),
                crypto::EncryptionTag::FolderKey,
            )
            .map_err(|e| {
                error!("Error encrypting folder key with share key: {e}");
                anyhow!("Error encrypting folder key with share key")
            })?;

            (encrypted, key_rotation)
        };

        // Build request
        let request = MoveFolderRequest {
            parent_folder_id: new_parent_id.as_ref().map(|id| id.to_string()),
            folder_keys: vec![MoveFolderKeyItem {
                key_rotation,
                folder_key: crate::utils::b64_encode(&encrypted_folder_key),
            }],
        };

        // Send request
        let req = PUT!("/pass/v1/share/{share_id}/folder/{folder_id}/move")
            .body_json(request)
            .context("Error creating move folder request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending move folder request")?;

        let _response: CodeResponse = assert_response!(res);

        // Clear folders cache since the folder's parent has changed
        self.clear_folders_cache(share_id).await;

        // The folder key itself hasn't changed, just its encryption parent
        // The cached decrypted key remains valid
        trace!("Folder {} moved, cached key still valid", folder_id);

        Ok(())
    }
}
