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
use async_lock::RwLock;
use pass_domain::{
    AccountCrypto, ClientFeatures, CoreEventStorage, CursorEntry, DataStorage, DecryptedFolderKey,
    DecryptedShareKey, FolderId, FolderKeyStorage, FsStorage, LocalKey, LocalKeyProvider,
    PgpCrypto, ShareId, ShareKeyStorage,
};
use pass_fs::InMemoryFsStorage;
use pass_pgp::{NativePgpCrypto, ProtonAccountCrypto};
use std::collections::HashMap;
use std::sync::Arc;

pub struct StaticKeyProvider {
    pub key: Vec<u8>,
}

#[async_trait::async_trait]
impl LocalKeyProvider for StaticKeyProvider {
    async fn get_key(&self) -> Result<LocalKey> {
        Ok(LocalKey::new(self.key.clone()))
    }
    async fn remove_key(&self) -> Result<()> {
        Ok(())
    }
}

pub type ShareKeyStorageType = HashMap<ShareId, Vec<DecryptedShareKey>>;

#[derive(Clone)]
pub struct InMemoryShareKeyStorage {
    storage: Arc<RwLock<ShareKeyStorageType>>,
}

impl InMemoryShareKeyStorage {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl ShareKeyStorage for InMemoryShareKeyStorage {
    async fn get_share_keys(&self, share_id: &ShareId) -> Result<Option<Vec<DecryptedShareKey>>> {
        let storage = self.storage.read().await;
        Ok(storage.get(share_id).cloned())
    }

    async fn store_share_keys(
        &self,
        share_id: &ShareId,
        share_keys: Vec<DecryptedShareKey>,
    ) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert(share_id.clone(), share_keys);
        Ok(())
    }
}

pub type FolderKeyStorageType = HashMap<(ShareId, FolderId), Vec<DecryptedFolderKey>>;

#[derive(Clone)]
pub struct InMemoryFolderKeyStorage {
    storage: Arc<RwLock<FolderKeyStorageType>>,
}

impl InMemoryFolderKeyStorage {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl FolderKeyStorage for InMemoryFolderKeyStorage {
    async fn get_folder_keys(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
    ) -> Result<Option<Vec<DecryptedFolderKey>>> {
        let storage = self.storage.read().await;
        Ok(storage.get(&(share_id.clone(), folder_id.clone())).cloned())
    }

    async fn store_folder_keys(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
        folder_keys: Vec<DecryptedFolderKey>,
    ) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.insert((share_id.clone(), folder_id.clone()), folder_keys);
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryCoreEventStorage {
    cursor: Arc<RwLock<Option<String>>>,
}

impl InMemoryCoreEventStorage {
    pub fn new() -> Self {
        Self {
            cursor: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl CoreEventStorage for InMemoryCoreEventStorage {
    async fn get_cursor(&self) -> Result<Option<CursorEntry>> {
        Ok(self
            .cursor
            .read()
            .await
            .clone()
            .map(|event_id| CursorEntry {
                event_id,
                updated_at: 0, // always stale so tests exercise the full sync path
            }))
    }

    async fn set_cursor(&self, event_id: &str) -> Result<()> {
        *self.cursor.write().await = Some(event_id.to_string());
        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryDataStorage {
    share_key_storage: Arc<dyn ShareKeyStorage>,
    folder_key_storage: Arc<dyn FolderKeyStorage>,
    core_event_storage: Arc<dyn CoreEventStorage>,
}

impl InMemoryDataStorage {
    pub fn new() -> Self {
        Self {
            share_key_storage: Arc::new(InMemoryShareKeyStorage::new()),
            folder_key_storage: Arc::new(InMemoryFolderKeyStorage::new()),
            core_event_storage: Arc::new(InMemoryCoreEventStorage::new()),
        }
    }
}

#[async_trait::async_trait]
impl DataStorage for InMemoryDataStorage {
    async fn get_share_key_storage(&self) -> Arc<dyn ShareKeyStorage> {
        self.share_key_storage.clone()
    }

    async fn get_folder_key_storage(&self) -> Arc<dyn FolderKeyStorage> {
        self.folder_key_storage.clone()
    }

    async fn get_core_event_storage(&self) -> Arc<dyn CoreEventStorage> {
        self.core_event_storage.clone()
    }
}

#[derive(Clone)]
pub struct TestClientFeatures {
    pub storage: Arc<InMemoryFsStorage>,
    pub key_provider: Arc<StaticKeyProvider>,
    pub data_storage: Arc<dyn DataStorage>,
}

impl TestClientFeatures {
    pub fn new(key: Vec<u8>) -> Self {
        Self {
            storage: Arc::new(InMemoryFsStorage::new()),
            key_provider: Arc::new(StaticKeyProvider { key }),
            data_storage: Arc::new(InMemoryDataStorage::new()),
        }
    }
}

#[async_trait::async_trait]
impl ClientFeatures for TestClientFeatures {
    async fn get_local_key_provider(&self) -> Result<Arc<dyn LocalKeyProvider>> {
        Ok(self.key_provider.clone())
    }

    async fn get_account_crypto(&self) -> Arc<dyn AccountCrypto> {
        Arc::new(ProtonAccountCrypto)
    }

    async fn get_fs(&self) -> Arc<dyn FsStorage> {
        self.storage.clone()
    }

    async fn get_pgp_crypto(&self) -> Arc<dyn PgpCrypto> {
        Arc::new(NativePgpCrypto)
    }

    async fn get_telemetry_handler(&self) -> Arc<dyn pass_domain::TelemetryHandler> {
        Arc::new(pass_domain::NoopTelemetryHandler)
    }

    async fn get_data_storage(&self) -> Result<Arc<dyn DataStorage>> {
        Ok(self.data_storage.clone())
    }

    async fn on_session_invalidated(&self) -> Result<()> {
        Ok(())
    }
}
