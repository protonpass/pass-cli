use crate::PassClient;
use crate::folder::list::FolderResponse;
use anyhow::{Context, Result, anyhow};
use muon::POST;
use pass_domain::{FolderData, FolderId, ShareId, crypto};

pub struct CreateFolderPayload {
    pub name: String,
    pub parent_folder_id: Option<FolderId>,
}

#[derive(serde::Serialize)]
struct CreateFolderRequest {
    #[serde(rename = "ParentFolderID", skip_serializing_if = "Option::is_none")]
    parent_folder_id: Option<String>,
    #[serde(rename = "ContentFormatVersion")]
    content_format_version: u32,
    #[serde(rename = "Content")]
    content: String,
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
    #[serde(rename = "FolderKey")]
    folder_key: String,
}

#[derive(serde::Deserialize)]
struct CreateFolderResponse {
    #[serde(rename = "Folder")]
    folder: FolderResponse,
}

impl PassClient {
    pub async fn create_folder(
        &self,
        share_id: &ShareId,
        payload: CreateFolderPayload,
    ) -> Result<FolderId> {
        // Generate a new 32-byte folder key
        let folder_key = crypto::generate_encryption_key();

        // Create and serialize folder content
        let folder_data = FolderData::new(payload.name);
        let serialized_content = folder_data
            .serialize()
            .context("Error serializing folder content")?;

        // Encrypt content with folder key
        let encrypted_content = crypto::encrypt(
            &serialized_content,
            &folder_key,
            crypto::EncryptionTag::FolderContent,
        )
        .map_err(|e| {
            error!("Error encrypting folder content: {e}");
            anyhow!("Error encrypting folder content")
        })?;

        // Determine which key to use for encrypting the folder key
        let (encrypted_folder_key, key_rotation) =
            if let Some(ref parent_folder_id) = payload.parent_folder_id {
                // Folder has a parent, encrypt with parent's folder key
                let parent_rev = self
                    .get_folder_revision(share_id, parent_folder_id)
                    .await
                    .context("Error getting parent folder")?;

                let parent_key = self
                    .get_opened_folder_key(share_id, parent_folder_id, parent_rev.key_rotation)
                    .await
                    .context("Error opening parent folder key")?;

                let encrypted = crypto::encrypt(
                    &folder_key,
                    parent_key.as_ref(),
                    crypto::EncryptionTag::FolderKey,
                )
                .map_err(|e| {
                    error!("Error encrypting folder key with parent key: {e}");
                    anyhow!("Error encrypting folder key with parent key")
                })?;

                (encrypted, parent_rev.key_rotation)
            } else {
                // Root folder, encrypt with share/vault key
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
                    &folder_key,
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
        let request = CreateFolderRequest {
            parent_folder_id: payload.parent_folder_id.as_ref().map(|id| id.to_string()),
            content_format_version: crate::constants::FOLDER_CONTENT_CONTENT_FORMAT_VERSION,
            content: crate::utils::b64_encode(&encrypted_content),
            key_rotation,
            folder_key: crate::utils::b64_encode(&encrypted_folder_key),
        };

        // Send request
        let req = POST!("/pass/v1/share/{share_id}/folder")
            .body_json(request)
            .context("Error creating folder request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending create folder request")?;

        let response: CreateFolderResponse = assert_response!(res);

        let folder_id = FolderId::new(response.folder.folder_id);

        // Store the folder key in storage (best effort)
        if let Ok(data_storage) = self.client_features.get_data_storage().await {
            let folder_key_storage = data_storage.get_folder_key_storage().await;
            let decrypted_key = pass_domain::DecryptedFolderKey::new(key_rotation, folder_key);
            let res = folder_key_storage
                .store_folder_keys(share_id, &folder_id, vec![decrypted_key])
                .await;
            if let Err(e) = res {
                warn!("Error storing folder key: {e:#}");
            }
        }

        // Clear folders cache since we created a new folder
        self.clear_folders_cache(share_id).await;

        Ok(folder_id)
    }
}
