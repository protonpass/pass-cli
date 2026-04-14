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

use anyhow::Result;
use std::path::Path;

#[async_trait::async_trait]
pub trait FsStorage {
    async fn get_file(&self, path: &Path) -> Result<Vec<u8>>;
    async fn file_exists(&self, path: &Path) -> Result<bool>;
    async fn store_file(&self, contents: Vec<u8>, path: &Path) -> Result<()>;
    async fn remove_file(&self, path: &Path) -> Result<()>;
}
