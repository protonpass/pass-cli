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

use crate::domain::ShareId;
use crate::{PassClient, PassClientContext};
use pass_derive::sdk_export;

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    /// Disables the in-memory cache and drops everything it holds. While disabled, every
    /// lookup misses and nothing is stored. The setting is shared by all clones of the client.
    #[sdk_export]
    pub async fn disable_cache(&self) {
        self.cache.set_enabled(false).await;
    }

    /// Enables the in-memory cache again. The cache is enabled by default.
    #[sdk_export]
    pub async fn enable_cache(&self) {
        self.cache.set_enabled(true).await;
    }

    /// Clears every entry of the in-memory cache without changing whether it is enabled.
    #[sdk_export]
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    /// Clears the cached data that belongs to a share: its items, folders, share keys and
    /// item revisions, plus the cached share and vault lists that include its details.
    #[sdk_export]
    pub async fn clear_cache_for_share(&self, share_id: &ShareId) {
        self.clear_items_cache(share_id).await;
        self.clear_folders_cache(share_id).await;
        self.clear_share_keys_cache(share_id).await;
        self.clear_item_revisions_cache(share_id).await;
        self.clear_shares_cache().await;
    }
}
