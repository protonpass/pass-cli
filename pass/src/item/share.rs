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

use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::Context;
use pass_domain::{ItemId, ShareId, ShareRole};

impl<C: PassClientContext> PassClient<C> {
    pub async fn share_item(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        email: &str,
        role: &ShareRole,
    ) -> anyhow::Result<()> {
        self.action_guard(PermissionAction::ShareVault).await?;

        let request = self
            .create_invites_request(share_id, email, role, Some(item_id.clone()))
            .await
            .context("Error creating invite to vault request")?;

        self.send_invite(share_id, request)
            .await
            .context("Error sending invite to item request")?;

        Ok(())
    }
}
