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

use crate::{TargetType, VaultData};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct InviteId(pub(crate) String);
display_for_basic!(InviteId);

impl InviteId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Invite {
    pub id: InviteId,
    pub token: String,
    pub target_type: TargetType,
    pub target_id: String,
    pub reminders: u8,
    pub inviter_email: String,
    pub invited_email: String,
    pub vault_data: Option<InviteVaultData>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct InviteVaultData {
    pub vault_data: VaultData,
    pub member_count: u32,
    pub item_count: u32,
}
