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

use pass_domain::{DataStorage, FolderKeyStorage, ShareKeyStorage};
use std::sync::Arc;

pub struct CliDataStorage {
    share_key_storage: Arc<dyn ShareKeyStorage>,
    folder_key_storage: Arc<dyn FolderKeyStorage>,
}

impl CliDataStorage {
    pub fn new(
        share_key_storage: Arc<dyn ShareKeyStorage>,
        folder_key_storage: Arc<dyn FolderKeyStorage>,
    ) -> Self {
        Self {
            share_key_storage,
            folder_key_storage,
        }
    }
}

#[async_trait::async_trait]
impl DataStorage for CliDataStorage {
    async fn get_share_key_storage(&self) -> Arc<dyn ShareKeyStorage> {
        self.share_key_storage.clone()
    }

    async fn get_folder_key_storage(&self) -> Arc<dyn FolderKeyStorage> {
        self.folder_key_storage.clone()
    }
}
