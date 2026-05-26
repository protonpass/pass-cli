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
use pass_db::{CoreEventCursorModel, DatabaseManager};
use pass_domain::CoreEventStorage;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct DatabaseCoreEventStorage {
    db: DatabaseManager,
    user_id: Arc<RwLock<Option<String>>>,
}

impl DatabaseCoreEventStorage {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            db,
            user_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn set_user_id(&self, user_id: Option<String>) {
        *self.user_id.write().await = user_id;
    }
}

#[async_trait::async_trait]
impl CoreEventStorage for DatabaseCoreEventStorage {
    async fn get_cursor(&self) -> Result<Option<String>> {
        let user_id = self.user_id.read().await.clone();
        let Some(user_id) = user_id else {
            return Ok(None);
        };
        CoreEventCursorModel::get(&self.db, &user_id).await
    }

    async fn set_cursor(&self, event_id: &str) -> Result<()> {
        let user_id = self.user_id.read().await.clone();
        let Some(user_id) = user_id else {
            warn!("No user_id set, skipping core event cursor storage");
            return Ok(());
        };
        CoreEventCursorModel::upsert(&self.db, &user_id, event_id).await
    }
}
