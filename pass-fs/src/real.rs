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
