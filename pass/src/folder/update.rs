use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_domain::{FolderData, FolderId, ShareId, crypto};

#[derive(serde::Serialize)]
struct UpdateFolderContent {
    #[serde(rename = "ContentFormatVersion")]
    content_format_version: u32,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
}

#[derive(serde::Serialize)]
struct UpdateFolderRequest {
    #[serde(rename = "Content")]
    content: UpdateFolderContent,
}

impl PassClient {
    pub async fn update_folder(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
        new_name: String,
    ) -> Result<()> {
        // Get the folder revision
        let folder_rev = self
            .get_folder_data(share_id, folder_id)
            .await
            .context("Error getting folder revision")?;

        // Open the folder key (will use cache if available)
        let folder_key = self
            .get_opened_folder_key(share_id, folder_id, folder_rev.key_rotation)
            .await
            .context("Error opening folder key")?;

        // Create new folder data with new name
        let folder_data = FolderData::new(new_name);
        let serialized_content = folder_data
            .serialize()
            .context("Error serializing folder content")?;

        // Encrypt content with the same folder key
        let encrypted_content = crypto::encrypt(
            &serialized_content,
            folder_key.as_ref(),
            crypto::EncryptionTag::FolderContent,
        )
        .map_err(|e| {
            error!("Error encrypting folder content: {e}");
            anyhow!("Error encrypting folder content")
        })?;

        // Build request
        let request = UpdateFolderRequest {
            content: UpdateFolderContent {
                content_format_version: crate::constants::FOLDER_CONTENT_CONTENT_FORMAT_VERSION,
                content: crate::utils::b64_encode(&encrypted_content),
                key_rotation: folder_rev.key_rotation,
            },
        };

        // Send request
        let req = PUT!("/pass/v1/share/{share_id}/folder/{folder_id}")
            .body_json(request)
            .context("Error creating update folder request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending update folder request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        // Clear folders cache since we updated a folder
        self.clear_folders_cache(share_id).await;

        Ok(())
    }
}
