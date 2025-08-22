use anyhow::Context;
use pass_domain::FsStorage;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct RealFsStorage {
    base_dir: PathBuf,
}

impl RealFsStorage {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[async_trait::async_trait]
impl FsStorage for RealFsStorage {
    async fn get_file(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        tokio::fs::read(self.base_dir.join(path))
            .await
            .context("Error reading file")
    }

    async fn file_exists(&self, path: &Path) -> anyhow::Result<bool> {
        match tokio::fs::metadata(self.base_dir.join(path)).await {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn store_file(&self, contents: Vec<u8>, path: &Path) -> anyhow::Result<()> {
        tokio::fs::write(self.base_dir.join(path), &contents)
            .await
            .context("Error writing file")?;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> anyhow::Result<()> {
        tokio::fs::remove_file(self.base_dir.join(path))
            .await
            .context("Error deleting file")
    }
}
