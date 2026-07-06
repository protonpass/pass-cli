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
use pass_db::{DatabaseManager, OrganizationPolicyModel};
use pass_domain::{
    OrganizationPolicyEntry, OrganizationPolicyStorage,
    models::organization_policy::OrganizationInfo,
};
use std::sync::Arc;
use tokio::sync::RwLock;

const ORGANIZATION_POLICY_CACHE_TTL_SECS: i64 = 8 * 60 * 60; // 8 hours

fn is_stale(updated_at: i64, now: i64) -> bool {
    now - updated_at >= ORGANIZATION_POLICY_CACHE_TTL_SECS
}

pub struct DatabaseOrganizationPolicyStorage {
    db: DatabaseManager,
    user_id: Arc<RwLock<Option<String>>>,
}

impl DatabaseOrganizationPolicyStorage {
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
impl OrganizationPolicyStorage for DatabaseOrganizationPolicyStorage {
    async fn get_policy(&self) -> Result<Option<OrganizationPolicyEntry>> {
        let user_id = self.user_id.read().await.clone();
        let Some(user_id) = user_id else {
            return Ok(None);
        };

        let Some(model_entry) = OrganizationPolicyModel::get(&self.db, &user_id).await? else {
            return Ok(None);
        };

        let now = jiff::Timestamp::now().as_second();
        if is_stale(model_entry.updated_at, now) {
            return Ok(None);
        }

        let policy: OrganizationInfo = serde_json::from_str(&model_entry.policy_json)
            .map_err(|e| anyhow::anyhow!("Error parsing cached organization policy: {}", e))?;

        Ok(Some(OrganizationPolicyEntry {
            policy,
            updated_at: model_entry.updated_at,
        }))
    }

    async fn set_policy(&self, policy: &OrganizationInfo) -> Result<()> {
        let user_id = self.user_id.read().await.clone();
        let Some(user_id) = user_id else {
            warn!("No user_id set, skipping organization policy storage");
            return Ok(());
        };
        OrganizationPolicyModel::upsert(&self.db, &user_id, policy).await
    }
}
