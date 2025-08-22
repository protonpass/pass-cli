use anyhow::{Context, Result};
use pass_domain::FsStorage;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// An in-memory implementation of FsStorage for testing purposes
#[derive(Clone)]
pub struct InMemoryFsStorage {
    files: Arc<RwLock<HashMap<PathBuf, Vec<u8>>>>,
}

impl InMemoryFsStorage {
    /// Creates a new empty InMemoryFsStorage
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates a new InMemoryFsStorage with initial files
    pub fn with_files(files: HashMap<PathBuf, Vec<u8>>) -> Self {
        Self {
            files: Arc::new(RwLock::new(files)),
        }
    }

    /// Returns the number of files currently stored
    pub async fn file_count(&self) -> usize {
        self.files.read().await.len()
    }

    /// Clears all files from storage
    pub async fn clear(&self) {
        self.files.write().await.clear();
    }

    /// Lists all file paths currently stored
    pub async fn list_files(&self) -> Vec<PathBuf> {
        self.files.read().await.keys().cloned().collect()
    }
}

impl Default for InMemoryFsStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl FsStorage for InMemoryFsStorage {
    async fn get_file(&self, path: &Path) -> Result<Vec<u8>> {
        let files = self.files.read().await;
        files
            .get(path)
            .cloned()
            .context(format!("File not found: {}", path.display()))
    }

    async fn file_exists(&self, path: &Path) -> Result<bool> {
        let files = self.files.read().await;
        Ok(files.contains_key(path))
    }

    async fn store_file(&self, contents: Vec<u8>, path: &Path) -> Result<()> {
        let mut files = self.files.write().await;
        files.insert(path.to_path_buf(), contents);
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        let mut files = self.files.write().await;
        files
            .remove(path)
            .map(|_| ())
            .context(format!("File not found: {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_new_storage_is_empty() {
        let storage = InMemoryFsStorage::new();
        assert_eq!(storage.file_count().await, 0);
        assert!(storage.list_files().await.is_empty());
    }

    #[tokio::test]
    async fn test_store_and_get_file() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("test.txt");
        let contents = b"Hello, world!".to_vec();

        // Store the file
        storage.store_file(contents.clone(), path).await.unwrap();
        assert_eq!(storage.file_count().await, 1);

        // Get the file back
        let retrieved = storage.get_file(path).await.unwrap();
        assert_eq!(retrieved, contents);
    }

    #[tokio::test]
    async fn test_file_exists() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("test.txt");
        let contents = b"test content".to_vec();

        // File should not exist initially
        assert!(!storage.file_exists(path).await.unwrap());

        // Store file and check it exists
        storage.store_file(contents, path).await.unwrap();
        assert!(storage.file_exists(path).await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_file() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("test.txt");
        let contents = b"test content".to_vec();

        // Store file
        storage.store_file(contents, path).await.unwrap();
        assert!(storage.file_exists(path).await.unwrap());
        assert_eq!(storage.file_count().await, 1);

        // Remove file
        storage.remove_file(path).await.unwrap();
        assert!(!storage.file_exists(path).await.unwrap());
        assert_eq!(storage.file_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_nonexistent_file_returns_error() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("nonexistent.txt");

        let result = storage.get_file(path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[tokio::test]
    async fn test_remove_nonexistent_file_returns_error() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("nonexistent.txt");

        let result = storage.remove_file(path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[tokio::test]
    async fn test_overwrite_file() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("test.txt");
        let contents1 = b"first content".to_vec();
        let contents2 = b"second content".to_vec();

        // Store first content
        storage.store_file(contents1, path).await.unwrap();
        let retrieved1 = storage.get_file(path).await.unwrap();
        assert_eq!(retrieved1, b"first content".to_vec());

        // Overwrite with second content
        storage.store_file(contents2.clone(), path).await.unwrap();
        let retrieved2 = storage.get_file(path).await.unwrap();
        assert_eq!(retrieved2, contents2);
        assert_eq!(storage.file_count().await, 1); // Should still be only one file
    }

    #[tokio::test]
    async fn test_multiple_files() {
        let storage = InMemoryFsStorage::new();
        let path1 = Path::new("file1.txt");
        let path2 = Path::new("dir/file2.txt");
        let contents1 = b"content 1".to_vec();
        let contents2 = b"content 2".to_vec();

        // Store multiple files
        storage.store_file(contents1.clone(), path1).await.unwrap();
        storage.store_file(contents2.clone(), path2).await.unwrap();

        assert_eq!(storage.file_count().await, 2);
        assert!(storage.file_exists(path1).await.unwrap());
        assert!(storage.file_exists(path2).await.unwrap());

        // Check contents
        assert_eq!(storage.get_file(path1).await.unwrap(), contents1);
        assert_eq!(storage.get_file(path2).await.unwrap(), contents2);

        // Check list_files contains both paths
        let files = storage.list_files().await;
        assert_eq!(files.len(), 2);
        assert!(files.contains(&path1.to_path_buf()));
        assert!(files.contains(&path2.to_path_buf()));
    }

    #[tokio::test]
    async fn test_with_files_constructor() {
        let mut initial_files = HashMap::new();
        initial_files.insert(PathBuf::from("file1.txt"), b"content 1".to_vec());
        initial_files.insert(PathBuf::from("file2.txt"), b"content 2".to_vec());

        let storage = InMemoryFsStorage::with_files(initial_files);

        assert_eq!(storage.file_count().await, 2);
        assert!(storage.file_exists(Path::new("file1.txt")).await.unwrap());
        assert!(storage.file_exists(Path::new("file2.txt")).await.unwrap());

        assert_eq!(
            storage.get_file(Path::new("file1.txt")).await.unwrap(),
            b"content 1".to_vec()
        );
        assert_eq!(
            storage.get_file(Path::new("file2.txt")).await.unwrap(),
            b"content 2".to_vec()
        );
    }

    #[tokio::test]
    async fn test_clear() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("test.txt");
        let contents = b"test content".to_vec();

        // Store file
        storage.store_file(contents, path).await.unwrap();
        assert_eq!(storage.file_count().await, 1);

        // Clear storage
        storage.clear().await;
        assert_eq!(storage.file_count().await, 0);
        assert!(!storage.file_exists(path).await.unwrap());
    }

    #[tokio::test]
    async fn test_clone_shares_storage() {
        let storage1 = InMemoryFsStorage::new();
        let storage2 = storage1.clone();
        let path = Path::new("test.txt");
        let contents = b"shared content".to_vec();

        // Store in first storage
        storage1.store_file(contents.clone(), path).await.unwrap();

        // Should be visible in cloned storage
        assert!(storage2.file_exists(path).await.unwrap());
        assert_eq!(storage2.get_file(path).await.unwrap(), contents);
        assert_eq!(storage2.file_count().await, 1);
    }

    #[tokio::test]
    async fn test_empty_file() {
        let storage = InMemoryFsStorage::new();
        let path = Path::new("empty.txt");
        let contents = Vec::new();

        storage.store_file(contents.clone(), path).await.unwrap();
        assert!(storage.file_exists(path).await.unwrap());
        assert_eq!(storage.get_file(path).await.unwrap(), contents);
    }
}
