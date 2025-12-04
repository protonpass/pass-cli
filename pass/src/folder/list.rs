use crate::PassClient;
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::{Folder, FolderData, FolderId, ShareId};
use std::collections::{HashMap, HashSet, VecDeque};

const PAGE_SIZE: usize = 100;

struct FoldersForShareCacheType;
type FoldersForShareCache = HashMap<ShareId, Vec<FolderResponse>>;

#[derive(Debug, Clone, serde::Deserialize)]
pub(crate) struct FolderResponse {
    #[serde(rename = "VaultID")]
    #[allow(dead_code)]
    pub vault_id: String,
    #[serde(rename = "FolderID")]
    pub folder_id: String,
    #[serde(rename = "ParentFolderID")]
    pub parent_folder_id: Option<String>,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
    #[serde(rename = "FolderKey")]
    pub folder_key: String,
    #[serde(rename = "ContentFormatVersion")]
    #[allow(dead_code)]
    pub content_format_version: u32,
    #[serde(rename = "Content")]
    pub content: String,
}

#[derive(Debug, serde::Deserialize)]
struct FoldersWrapper {
    #[serde(rename = "Folders")]
    folders: Vec<FolderResponse>,
    #[serde(rename = "LastToken")]
    last_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ListFoldersResponse {
    #[serde(rename = "Folders")]
    folders: FoldersWrapper,
}

#[derive(Debug, serde::Deserialize)]
struct GetFolderResponse {
    #[serde(rename = "Folder")]
    pub folder: FolderResponse,
}

// Performs topological sort on folder revisions, filtering out dangling folders.
// Returns folder IDs in topological order (parents before children).
fn topological_sort_folders(revisions: &[FolderResponse]) -> Vec<String> {
    if revisions.is_empty() {
        return Vec::new();
    }

    // Build map for quick lookup
    let revision_map: HashMap<String, &FolderResponse> =
        revisions.iter().map(|r| (r.folder_id.clone(), r)).collect();

    // Build adjacency list for topological sort (parent -> children)
    let mut children: HashMap<Option<String>, Vec<String>> = HashMap::new();

    for rev in revisions {
        let folder_id = rev.folder_id.clone();
        let parent_id = rev.parent_folder_id.clone();

        // Check if parent exists (if it should have one)
        if let Some(ref parent) = parent_id
            && !revision_map.contains_key(parent)
        {
            // Dangling folder - parent doesn't exist, skip it
            warn!("Found dangling folder [folder_id={folder_id}] [missing_parent_id={parent}]");
            continue;
        }

        children.entry(parent_id).or_default().push(folder_id);
    }

    // Topological sort: process folders level by level from root to leaves
    let mut sorted_ids = Vec::new();
    let mut queue = VecDeque::new();

    // Start with root folders (no parent)
    if let Some(root_folders) = children.get(&None) {
        for folder_id in root_folders {
            queue.push_back(folder_id.clone());
        }
    }

    while let Some(folder_id) = queue.pop_front() {
        sorted_ids.push(folder_id.clone());

        // Add children to queue
        if let Some(child_folders) = children.get(&Some(folder_id.clone())) {
            for child_id in child_folders {
                queue.push_back(child_id.clone());
            }
        }
    }

    sorted_ids
}

impl PassClient {
    pub(crate) async fn list_all_folder_revisions(
        &self,
        share_id: &ShareId,
    ) -> Result<Vec<FolderResponse>> {
        self.list_all_folder_revisions_impl(share_id, false).await
    }

    #[allow(dead_code)]
    pub(crate) async fn list_all_folder_revisions_force_refresh(
        &self,
        share_id: &ShareId,
    ) -> Result<Vec<FolderResponse>> {
        self.list_all_folder_revisions_impl(share_id, true).await
    }

    async fn list_all_folder_revisions_impl(
        &self,
        share_id: &ShareId,
        force_refresh: bool,
    ) -> Result<Vec<FolderResponse>> {
        // Check cache first (unless force_refresh is set)
        if !force_refresh {
            self.cache
                .ensure_has_value(FoldersForShareCacheType, FoldersForShareCache::new)
                .await;

            let cached: Option<FoldersForShareCache> =
                self.cache.get(FoldersForShareCacheType).await;
            if let Some(cached) = cached
                && let Some(cached_folders) = cached.get(share_id)
            {
                trace!("Returning cached folder revisions for share {share_id}");
                return Ok(cached_folders.clone());
            }
        }

        // Not in cache or force refresh, fetch from API
        trace!("Fetching folder revisions from API for share {share_id}");
        let mut all_revisions = Vec::new();
        let mut last_token: Option<String> = None;

        loop {
            let mut req = GET!("/pass/v1/share/{share_id}/folder")
                .query(("PageSize", format!("{}", PAGE_SIZE)));

            if let Some(token) = &last_token {
                req = req.query(("Since", token.clone()));
            }

            let res = self
                .send(req)
                .await
                .context("Error sending list folders request")?;

            let response: ListFoldersResponse = assert_response!(res);
            let folders_wrapper = response.folders;

            all_revisions.extend(folders_wrapper.folders);

            match folders_wrapper.last_token {
                Some(token) if !token.is_empty() => {
                    last_token = Some(token);
                }
                _ => break,
            }
        }

        // Store in cache
        self.cache
            .update(
                FoldersForShareCacheType,
                |cache: &mut FoldersForShareCache| {
                    cache.insert(share_id.clone(), all_revisions.clone());
                },
            )
            .await;

        Ok(all_revisions)
    }

    pub async fn list_folders(&self, share_id: &ShareId) -> Result<Vec<Folder>> {
        // Get all folder revisions with pagination
        let all_revisions = self
            .list_all_folder_revisions(share_id)
            .await
            .context("Error fetching all folder revisions")?;

        // Build folder structure and open all valid folders
        self.build_folder_structure(share_id, all_revisions)
            .await
            .context("Error building folder structure")
    }

    pub(crate) async fn get_folder_data(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
    ) -> Result<FolderResponse> {
        // Try to get from cache first by fetching all folders (which may be cached)
        match self.list_all_folder_revisions(share_id).await {
            Ok(revisions) => {
                // Look for the specific folder in the cached/fetched list
                if let Some(folder_rev) = revisions
                    .into_iter()
                    .find(|r| r.folder_id == folder_id.value())
                {
                    trace!("Found folder {} in cache/list", folder_id);
                    return Ok(folder_rev);
                }
            }
            Err(e) => {
                debug!("Error listing folders, falling back to individual get: {e:#}");
            }
        }

        // Not in list/cache, fetch individual folder
        trace!("Fetching individual folder {} from API", folder_id);
        let req = GET!("/pass/v1/share/{share_id}/folder/{folder_id}");
        let res = self
            .send(req)
            .await
            .context("Error sending get folder request")?;

        let response: GetFolderResponse = assert_response!(res);
        let folder_rev = response.folder;

        // Add the fetched folder to cache
        self.cache
            .ensure_has_value(FoldersForShareCacheType, FoldersForShareCache::new)
            .await;
        self.cache
            .update(
                FoldersForShareCacheType,
                |cache: &mut FoldersForShareCache| {
                    cache
                        .entry(share_id.clone())
                        .or_default()
                        .push(folder_rev.clone());
                },
            )
            .await;
        trace!("Added folder {} to cache", folder_id);

        Ok(folder_rev)
    }

    async fn build_folder_structure(
        &self,
        share_id: &ShareId,
        mut revisions: Vec<FolderResponse>,
    ) -> Result<Vec<Folder>> {
        if revisions.is_empty() {
            return Ok(Vec::new());
        }

        // Build maps for quick lookup
        let mut revision_map: HashMap<String, FolderResponse> = revisions
            .iter()
            .map(|r| (r.folder_id.clone(), r.clone()))
            .collect();

        // Find dangling folders (folders whose parent is missing)
        let mut missing_parents = HashSet::new();
        for rev in &revisions {
            if let Some(parent_id) = &rev.parent_folder_id
                && !revision_map.contains_key(parent_id)
            {
                missing_parents.insert(parent_id.clone());
            }
        }

        // Try to fetch missing parents
        for parent_id in missing_parents {
            match self
                .get_folder_data(share_id, &FolderId::new(parent_id.clone()))
                .await
            {
                Ok(parent_rev) => {
                    trace!("Fetched missing parent folder {}", parent_id);
                    revision_map.insert(parent_id.clone(), parent_rev.clone());
                    revisions.push(parent_rev);
                }
                Err(e) => {
                    debug!(
                        "Could not fetch missing parent folder {}: {}. Will discard this branch.",
                        parent_id, e
                    );
                }
            }
        }

        // Perform topological sort to get folders in correct order (parents before children)
        let sorted_ids = topological_sort_folders(&revisions);

        // Open folders in topological order (parents before children)
        // This allows us to use cached parent keys when opening children
        let mut opened_folders = Vec::new();

        for folder_id in sorted_ids {
            if let Some(rev) = revision_map.get(&folder_id) {
                match self.open_folder(rev, share_id).await {
                    Ok(folder) => {
                        opened_folders.push(folder);
                    }
                    Err(e) => {
                        warn!("Error opening folder {}: {}. Skipping.", folder_id, e);
                    }
                }
            }
        }

        Ok(opened_folders)
    }

    async fn open_folder(&self, folder_rev: &FolderResponse, share_id: &ShareId) -> Result<Folder> {
        // Open the folder key
        let folder_key = self
            .get_opened_folder_key(
                share_id,
                &FolderId::new(folder_rev.folder_id.clone()),
                folder_rev.key_rotation,
            )
            .await
            .context("Error opening folder key")?;

        // Decrypt and deserialize folder content
        let content = self
            .open_folder_content(folder_key.as_ref(), folder_rev)
            .await
            .context("Error opening folder content")?;

        Ok(Folder {
            id: FolderId::new(folder_rev.folder_id.clone()),
            share_id: share_id.clone(),
            parent_folder_id: folder_rev
                .parent_folder_id
                .as_ref()
                .map(|id| FolderId::new(id.clone())),
            content,
        })
    }

    async fn open_folder_content(
        &self,
        folder_key: &[u8],
        folder_rev: &FolderResponse,
    ) -> Result<FolderData> {
        match folder_rev.content_format_version {
            1 => {
                self.open_folder_content_cfv1(folder_key, &folder_rev.content)
                    .await
            }
            _ => Err(anyhow::anyhow!(
                "Unsupported content format version for folder content: {}",
                folder_rev.content_format_version
            )),
        }
    }

    async fn open_folder_content_cfv1(
        &self,
        folder_key: &[u8],
        encrypted_content: &str,
    ) -> Result<FolderData> {
        let encrypted_bytes =
            crate::utils::b64_decode(encrypted_content).context("Error decoding folder content")?;

        let decrypted = pass_domain::crypto::decrypt(
            &encrypted_bytes,
            folder_key,
            pass_domain::crypto::EncryptionTag::FolderContent,
        )
        .map_err(|e| {
            error!("Error decrypting folder content: {e}");
            anyhow::anyhow!("Error decrypting folder content")
        })?;

        FolderData::deserialize(&decrypted).context("Error deserializing folder content")
    }

    pub(crate) async fn clear_folders_cache(&self, share_id: &ShareId) {
        self.cache
            .update(
                FoldersForShareCacheType,
                |cache: &mut FoldersForShareCache| {
                    cache.remove(share_id);
                },
            )
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_folder_revision(folder_id: &str, parent_folder_id: Option<&str>) -> FolderResponse {
        FolderResponse {
            vault_id: "vault1".to_string(),
            folder_id: folder_id.to_string(),
            parent_folder_id: parent_folder_id.map(|s| s.to_string()),
            key_rotation: 1,
            folder_key: "encrypted_key".to_string(),
            content_format_version: 1,
            content: "encrypted_content".to_string(),
        }
    }

    #[test]
    fn test_topological_sort_empty() {
        let revisions: Vec<FolderResponse> = vec![];
        let result = topological_sort_folders(&revisions);
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_topological_sort_single_root() {
        let revisions = vec![make_folder_revision("folder1", None)];
        let result = topological_sort_folders(&revisions);
        assert_eq!(result, vec!["folder1"]);
    }

    #[test]
    fn test_topological_sort_multiple_roots() {
        let revisions = vec![
            make_folder_revision("folder1", None),
            make_folder_revision("folder2", None),
            make_folder_revision("folder3", None),
        ];
        let result = topological_sort_folders(&revisions);

        // All should be included
        assert_eq!(result.len(), 3);
        assert!(result.contains(&"folder1".to_string()));
        assert!(result.contains(&"folder2".to_string()));
        assert!(result.contains(&"folder3".to_string()));
    }

    #[test]
    fn test_topological_sort_simple_hierarchy() {
        let revisions = vec![
            make_folder_revision("root", None),
            make_folder_revision("child1", Some("root")),
            make_folder_revision("child2", Some("root")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result.len(), 3);
        // Root must come first
        assert_eq!(result[0], "root");
        // Children come after root (order between children not guaranteed)
        assert!(result[1..].contains(&"child1".to_string()));
        assert!(result[1..].contains(&"child2".to_string()));
    }

    #[test]
    fn test_topological_sort_deep_hierarchy() {
        let revisions = vec![
            make_folder_revision("root", None),
            make_folder_revision("level1", Some("root")),
            make_folder_revision("level2", Some("level1")),
            make_folder_revision("level3", Some("level2")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result, vec!["root", "level1", "level2", "level3"]);
    }

    #[test]
    fn test_topological_sort_complex_tree() {
        // Tree structure:
        //       root
        //      /    \
        //   child1  child2
        //   /    \
        // gc1   gc2
        let revisions = vec![
            make_folder_revision("root", None),
            make_folder_revision("child1", Some("root")),
            make_folder_revision("child2", Some("root")),
            make_folder_revision("gc1", Some("child1")),
            make_folder_revision("gc2", Some("child1")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result.len(), 5);
        // Root first
        assert_eq!(result[0], "root");

        // Get indices
        let idx_child1 = result.iter().position(|x| x == "child1").unwrap();
        let idx_child2 = result.iter().position(|x| x == "child2").unwrap();
        let idx_gc1 = result.iter().position(|x| x == "gc1").unwrap();
        let idx_gc2 = result.iter().position(|x| x == "gc2").unwrap();

        // Child1 and child2 come after root
        assert!(idx_child1 > 0);
        assert!(idx_child2 > 0);

        // Grandchildren come after child1
        assert!(idx_gc1 > idx_child1);
        assert!(idx_gc2 > idx_child1);
    }

    #[test]
    fn test_topological_sort_dangling_folder_excluded() {
        // Folder with missing parent should be excluded
        let revisions = vec![
            make_folder_revision("root", None),
            make_folder_revision("child", Some("root")),
            make_folder_revision("dangling", Some("nonexistent")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec!["root", "child"]);
        // Dangling folder should not be in result
        assert!(!result.contains(&"dangling".to_string()));
    }

    #[test]
    fn test_topological_sort_dangling_branch() {
        // Entire branch with missing root should be excluded
        let revisions = vec![
            make_folder_revision("root", None),
            make_folder_revision("valid_child", Some("root")),
            make_folder_revision("dangling", Some("missing_parent")),
            make_folder_revision("dangling_child", Some("dangling")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec!["root", "valid_child"]);
        // Dangling folder and its child should not be in result
        assert!(!result.contains(&"dangling".to_string()));
        assert!(!result.contains(&"dangling_child".to_string()));
    }

    #[test]
    fn test_topological_sort_multiple_trees() {
        // Multiple independent trees
        let revisions = vec![
            make_folder_revision("root1", None),
            make_folder_revision("root1_child", Some("root1")),
            make_folder_revision("root2", None),
            make_folder_revision("root2_child", Some("root2")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result.len(), 4);

        // Get indices
        let idx_root1 = result.iter().position(|x| x == "root1").unwrap();
        let idx_root1_child = result.iter().position(|x| x == "root1_child").unwrap();
        let idx_root2 = result.iter().position(|x| x == "root2").unwrap();
        let idx_root2_child = result.iter().position(|x| x == "root2_child").unwrap();

        // Each child comes after its parent
        assert!(idx_root1_child > idx_root1);
        assert!(idx_root2_child > idx_root2);
    }

    #[test]
    fn test_topological_sort_unordered_input() {
        // Input in random order, should still produce correct topological order
        let revisions = vec![
            make_folder_revision("level3", Some("level2")),
            make_folder_revision("level1", Some("root")),
            make_folder_revision("root", None),
            make_folder_revision("level2", Some("level1")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result, vec!["root", "level1", "level2", "level3"]);
    }

    #[test]
    fn test_topological_sort_mixed_valid_and_dangling() {
        let revisions = vec![
            make_folder_revision("root", None),
            make_folder_revision("valid1", Some("root")),
            make_folder_revision("dangling1", Some("missing1")),
            make_folder_revision("valid2", Some("valid1")),
            make_folder_revision("dangling2", Some("missing2")),
        ];
        let result = topological_sort_folders(&revisions);

        assert_eq!(result.len(), 3);
        assert_eq!(result, vec!["root", "valid1", "valid2"]);
    }
}
