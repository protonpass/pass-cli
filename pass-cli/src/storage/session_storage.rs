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

use anyhow::{Context, Result, anyhow};
use pass_auth::SessionStorage;
use std::path::PathBuf;

pub struct FileSystemSessionStorage {
    file_path: PathBuf,
}

impl FileSystemSessionStorage {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    async fn ensure_session_file_not_symlink(&self) -> Result<()> {
        match std::fs::symlink_metadata(&self.file_path) {
            Ok(metadata) if metadata.is_symlink() => Err(anyhow!(
                "Session file is a symlink, which is not allowed for security reasons"
            )),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(anyhow!("Error reading file metadata: {e}")),
            _ => Ok(()),
        }
    }
}

#[async_trait::async_trait]
impl SessionStorage for FileSystemSessionStorage {
    async fn load(&self) -> Result<Option<Vec<u8>>> {
        if !self.file_path.exists() || !self.file_path.is_file() {
            return Ok(None);
        }

        self.ensure_session_file_not_symlink().await?;

        let contents = std::fs::read(&self.file_path).context("Error reading session file")?;

        Ok(Some(contents))
    }

    async fn save(&self, data: &[u8]) -> Result<()> {
        self.ensure_session_file_not_symlink().await?;

        #[cfg(not(target_os = "windows"))]
        {
            use tokio::io::AsyncWriteExt;
            let mut options = tokio::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true).mode(0o600);
            let mut file = options
                .open(&self.file_path)
                .await
                .context("Error opening file with secure permissions")?;
            file.write_all(data)
                .await
                .context("Error writing session file")?;
        }

        #[cfg(target_os = "windows")]
        {
            tokio::fs::write(&self.file_path, data)
                .await
                .context("Error writing session file")?;
            // Restrict to current user only, mirroring the Unix 0600 permissions above.
            if let Err(e) =
                crate::platform::windows_permissions::restrict_file_to_current_user(&self.file_path)
            {
                warn!("Failed to restrict session file permissions: {e:#}");
            }
        }

        Ok(())
    }

    async fn exists(&self) -> bool {
        self.file_path.exists() && self.file_path.is_file()
    }

    async fn delete(&self) -> Result<()> {
        if self.file_path.exists() {
            tokio::fs::remove_file(&self.file_path)
                .await
                .context("Error deleting session file")?;
        }
        Ok(())
    }
}
