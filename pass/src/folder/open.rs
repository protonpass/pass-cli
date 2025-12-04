use crate::PassClient;
use crate::folder::list::FolderResponse;
use anyhow::{Context, Result, anyhow};
use pass_domain::{DecryptedFolderKey, FolderId, ShareId, crypto};

impl PassClient {
    /// Get the name of a folder using existing caches and methods
    pub async fn get_folder_name(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
    ) -> Result<String> {
        // Get the folder revision (may hit API)
        let folder_rev = self
            .get_folder_revision(share_id, folder_id)
            .await
            .context("Error getting folder revision")?;

        // Open the folder key (uses cache if available)
        let folder_key = self
            .get_opened_folder_key(share_id, folder_id, folder_rev.key_rotation)
            .await
            .context("Error opening folder key")?;

        // Decrypt and deserialize the folder content to get the name
        let encrypted_content = crate::utils::b64_decode(&folder_rev.content)
            .context("Error decoding folder content")?;

        let decrypted = crypto::decrypt(
            &encrypted_content,
            folder_key.as_ref(),
            crypto::EncryptionTag::FolderContent,
        )
        .map_err(|e| {
            error!("Error decrypting folder content: {e}");
            anyhow!("Error decrypting folder content")
        })?;

        let folder_data = pass_domain::FolderData::deserialize(&decrypted)
            .context("Error deserializing folder content")?;

        Ok(folder_data.name)
    }

    pub(crate) async fn get_opened_folder_key(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
        key_rotation: u8,
    ) -> Result<DecryptedFolderKey> {
        // Try to get from storage first
        if let Ok(data_storage) = self.client_features.get_data_storage().await {
            let folder_key_storage = data_storage.get_folder_key_storage().await;

            if let Ok(Some(cached_keys)) = folder_key_storage
                .get_folder_keys(share_id, folder_id)
                .await
                && let Some(cached_key) = cached_keys
                    .into_iter()
                    .find(|k| k.key_rotation == key_rotation)
            {
                trace!(
                    "Using cached decrypted folder key from database for folder {} rotation {}",
                    folder_id, key_rotation
                );
                return Ok(cached_key);
            }
        }

        trace!(
            "Folder key not in cache, fetching and opening for folder {} rotation {}",
            folder_id, key_rotation
        );

        let folder_rev = self
            .get_folder_revision(share_id, folder_id)
            .await
            .context("Error getting folder revision")?;

        let opened_key = self
            .open_folder_key_from_api(share_id, &folder_rev)
            .await
            .context("Error opening folder key from API")?;

        // Store in storage for future use (best effort, don't fail on error)
        if let Ok(data_storage) = self.client_features.get_data_storage().await {
            let folder_key_storage = data_storage.get_folder_key_storage().await;
            let res = folder_key_storage
                .store_folder_keys(share_id, folder_id, vec![opened_key.clone()])
                .await;
            if let Err(e) = res {
                warn!("Error storing folder key: {e:#}");
            }
        }

        Ok(opened_key)
    }

    async fn open_folder_key_from_api(
        &self,
        share_id: &ShareId,
        folder_rev: &FolderResponse,
    ) -> Result<DecryptedFolderKey> {
        use std::collections::HashMap;

        // Fetch all folders for this share at once (paginated internally)
        let all_revisions = self
            .list_all_folder_revisions(share_id)
            .await
            .context("Error fetching all folder revisions")?;

        // Build a map for quick lookup
        let revision_map: HashMap<String, &FolderResponse> = all_revisions
            .iter()
            .map(|r| (r.folder_id.clone(), r))
            .collect();

        // Build the path from root to target folder
        let mut path = Vec::new();
        let mut current_id = Some(folder_rev.folder_id.clone());

        // Walk backwards from target to root
        while let Some(folder_id) = current_id {
            let rev = revision_map
                .get(&folder_id)
                .ok_or_else(|| anyhow!("Folder {} not found in share", folder_id))?;

            path.push((*rev).clone());
            current_id = rev.parent_folder_id.clone();
        }

        // Reverse to get path from root to target
        path.reverse();

        // Open keys iteratively starting from root
        // This allows us to use cached keys for parents
        let mut current_key: Option<DecryptedFolderKey> = None;

        for (i, folder) in path.iter().enumerate() {
            // Check cache first for each folder in the path
            let folder_id_obj = FolderId::new(folder.folder_id.clone());
            if let Ok(data_storage) = self.client_features.get_data_storage().await {
                let folder_key_storage = data_storage.get_folder_key_storage().await;

                if let Ok(Some(cached_keys)) = folder_key_storage
                    .get_folder_keys(share_id, &folder_id_obj)
                    .await
                    && let Some(cached_key) = cached_keys
                        .into_iter()
                        .find(|k| k.key_rotation == folder.key_rotation)
                {
                    trace!("Using cached key for folder {} in path", folder.folder_id);
                    current_key = Some(cached_key);
                    continue;
                }
            }

            // Not in cache, decrypt it
            let encrypted_folder_key = crate::utils::b64_decode(&folder.folder_key)
                .context("Error decoding folder key")?;

            let decrypted_key = if i == 0 {
                // First folder (root), decrypt with share key
                let opened_share_key = self
                    .get_opened_share_key_by_rotation(share_id, folder.key_rotation)
                    .await
                    .context("Error opening share key")?;

                crypto::decrypt(
                    &encrypted_folder_key,
                    opened_share_key.as_ref(),
                    crypto::EncryptionTag::FolderKey,
                )
                .map_err(|e| {
                    error!("Error decrypting folder key with share key: {e}");
                    anyhow!("Error decrypting folder key with share key")
                })?
            } else {
                // Decrypt with parent folder key
                let parent_key = current_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("Parent key not available"))?;

                crypto::decrypt(
                    &encrypted_folder_key,
                    parent_key.as_ref(),
                    crypto::EncryptionTag::FolderKey,
                )
                .map_err(|e| {
                    error!("Error decrypting folder key with parent key: {e}");
                    anyhow!("Error decrypting folder key with parent key")
                })?
            };

            let decrypted_folder_key = DecryptedFolderKey::new(folder.key_rotation, decrypted_key);

            // Store in cache for future use (best effort)
            if let Ok(data_storage) = self.client_features.get_data_storage().await {
                let folder_key_storage = data_storage.get_folder_key_storage().await;
                let _ = folder_key_storage
                    .store_folder_keys(share_id, &folder_id_obj, vec![decrypted_folder_key.clone()])
                    .await;
            }

            current_key = Some(decrypted_folder_key);
        }

        current_key.ok_or_else(|| anyhow!("Failed to decrypt folder key"))
    }
}
