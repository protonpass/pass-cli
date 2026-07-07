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

use crate::models::organization_policy::OrganizationInfo;
use anyhow::Result;

pub struct OrganizationPolicyEntry {
    pub policy: OrganizationInfo,
    pub updated_at: i64,
}

#[async_trait::async_trait]
pub trait OrganizationPolicyStorage: Send + Sync {
    async fn get_policy(&self) -> Result<Option<OrganizationPolicyEntry>>;
    async fn set_policy(&self, policy: &OrganizationInfo) -> Result<()>;
}
