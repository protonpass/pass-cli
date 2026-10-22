/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::domain::{DecryptedFolderKey, FolderId, ShareId, ShareType, crypto};
use crate::folder::list::FolderResponse;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use pass_derive::sdk_export;

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    /// Get the name of a folder using existing caches and methods
    #[sdk_export]
    pub async fn get_folder_name(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
    ) -> Result<String> {
        // Get the folder revision (may hit API)
        let folder_rev = self
            .get_folder_data(share_id, folder_id)
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
            error!("Error decrypting folder content: {e:#}");
            anyhow!("Error decrypting folder content")
        })?;

        let folder_data = crate::domain::FolderData::deserialize(&decrypted)
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
            .get_folder_data(share_id, folder_id)
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

        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        let share_root_folder_id = match share.share_type {
            ShareType::Folder { folder_id, .. } => Some(folder_id),
            _ => None,
        };

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

            if share_root_folder_id
                .as_ref()
                .is_some_and(|id| id.value() == folder_id)
            {
                break;
            }

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

            let is_share_root = i == 0
                && share_root_folder_id
                    .as_ref()
                    .is_some_and(|id| id.value() == folder.folder_id);

            let decrypted_key = if is_share_root {
                let opened_share_key = self
                    .get_opened_share_key_by_rotation(share_id, folder.key_rotation)
                    .await
                    .context("Error opening share key")?;

                opened_share_key.value()
            } else if i == 0 {
                // First folder (root of the vault), decrypt with share key
                let encrypted_folder_key = crate::utils::b64_decode(&folder.folder_key)
                    .context("Error decoding folder key")?;

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
                    error!("Error decrypting folder key with share key: {e:#}");
                    anyhow!("Error decrypting folder key with share key")
                })?
            } else {
                // Decrypt with parent folder key
                let encrypted_folder_key = crate::utils::b64_decode(&folder.folder_key)
                    .context("Error decoding folder key")?;

                let parent_key = current_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("Parent key not available"))?;

                crypto::decrypt(
                    &encrypted_folder_key,
                    parent_key.as_ref(),
                    crypto::EncryptionTag::FolderKey,
                )
                .map_err(|e| {
                    error!("Error decrypting folder key with parent key: {e:#}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::TargetType;
    use crate::folder::list::{FoldersWrapper, ListFoldersResponse};
    use crate::share::list::ShareResponse;
    use crate::test_tools::*;

    fn setup_folder_share(server: &ProtonAPI, share_id: &str, folder_id: &str, vault_id: &str) {
        let share_response = ShareResponse {
            share_id: share_id.to_string(),
            address_id: TEST_ADDRESS_ID.to_string(),
            vault_id: vault_id.to_string(),
            target_type: TargetType::Folder.value(),
            target_id: folder_id.to_string(),
            owner: false,
            permission: 0,
            share_role_id: "1".to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 0,
            group_id: None,
            flags: 0,
        };
        let share_response_clone = share_response.clone();
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}", share_id),
            move |_| success(share_response_clone.clone()),
        );
        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(crate::share::list::GetSharesResponse {
                shares: vec![share_response.clone()],
            })
        });
    }

    fn setup_folders(server: &ProtonAPI, share_id: &str, folders: Vec<FolderResponse>) {
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/folder", share_id),
            move |_| {
                success(ListFoldersResponse {
                    folders: FoldersWrapper {
                        folders: folders.clone(),
                        last_token: None,
                    },
                })
            },
        );
    }

    fn make_folder_response(
        folder_id: &str,
        parent_folder_id: Option<&str>,
        key_rotation: u8,
        folder_key: &[u8],
    ) -> FolderResponse {
        FolderResponse {
            vault_id: TEST_VAULT_ID.to_string(),
            folder_id: folder_id.to_string(),
            parent_folder_id: parent_folder_id.map(|s| s.to_string()),
            key_rotation,
            folder_key: crate::utils::b64_encode(folder_key),
            content_format_version: 1,
            content: "".to_string(),
        }
    }

    #[muon_test::test]
    async fn test_get_opened_folder_key_for_folder_share_root(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "FOLDER_SHARE_ID";
        const ROOT_FOLDER_ID: &str = "ROOT_FOLDER_ID";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_folder_share(&api, SHARE_ID, ROOT_FOLDER_ID, TEST_VAULT_ID);

        let folder_key_raw = crypto::generate_encryption_key();
        let encrypted_share_key = client.encrypt_for_user_key(folder_key_raw.clone()).await;
        api.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/key", SHARE_ID),
            move |_| {
                success(crate::share::keys::GetShareKeysResponse {
                    keys: crate::share::keys::ShareKeyList {
                        keys: vec![crate::share::keys::ShareKeyResponse {
                            key_rotation: 1,
                            key: crate::utils::b64_encode(&encrypted_share_key),
                            create_time: 123456789,
                        }],
                        total: 1,
                    },
                })
            },
        );

        let unreachable_wrapped_bytes = crypto::generate_encryption_key();
        setup_folders(
            &api,
            SHARE_ID,
            vec![make_folder_response(
                ROOT_FOLDER_ID,
                None,
                1,
                &unreachable_wrapped_bytes,
            )],
        );

        let opened = client
            .get_opened_folder_key(
                &share_id!(SHARE_ID),
                &FolderId::new(ROOT_FOLDER_ID.to_string()),
                1,
            )
            .await
            .expect("Should open the folder share's root folder key");

        assert_eq!(folder_key_raw, opened.value());
    }

    #[muon_test::test]
    async fn test_get_opened_folder_key_for_nested_subfolder_via_folder_share(
        server: muon_test::Server,
    ) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "FOLDER_SHARE_ID";
        const ROOT_FOLDER_ID: &str = "ROOT_FOLDER_ID";
        const CHILD_FOLDER_ID: &str = "CHILD_FOLDER_ID";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_folder_share(&api, SHARE_ID, ROOT_FOLDER_ID, TEST_VAULT_ID);

        let root_key_raw = crypto::generate_encryption_key();
        let encrypted_share_key = client.encrypt_for_user_key(root_key_raw.clone()).await;
        api.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/key", SHARE_ID),
            move |_| {
                success(crate::share::keys::GetShareKeysResponse {
                    keys: crate::share::keys::ShareKeyList {
                        keys: vec![crate::share::keys::ShareKeyResponse {
                            key_rotation: 1,
                            key: crate::utils::b64_encode(&encrypted_share_key),
                            create_time: 123456789,
                        }],
                        total: 1,
                    },
                })
            },
        );

        let child_key_raw = crypto::generate_encryption_key();
        let encrypted_child_key = crypto::encrypt(
            &child_key_raw,
            &root_key_raw,
            crypto::EncryptionTag::FolderKey,
        )
        .expect("Error encrypting child folder key");

        setup_folders(
            &api,
            SHARE_ID,
            vec![
                make_folder_response(ROOT_FOLDER_ID, None, 1, &crypto::generate_encryption_key()),
                make_folder_response(
                    CHILD_FOLDER_ID,
                    Some(ROOT_FOLDER_ID),
                    1,
                    &encrypted_child_key,
                ),
            ],
        );

        let opened = client
            .get_opened_folder_key(
                &share_id!(SHARE_ID),
                &FolderId::new(CHILD_FOLDER_ID.to_string()),
                1,
            )
            .await
            .expect("Should open the nested subfolder key via the folder share");

        assert_eq!(child_key_raw, opened.value());
    }
}
