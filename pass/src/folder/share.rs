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

use crate::domain::{FolderId, ShareId, ShareRole};
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::Context;
use pass_derive::sdk_export;

#[sdk_export]
impl<C: PassClientContext> PassClient<C> {
    #[sdk_export]
    pub async fn share_folder(
        &self,
        share_id: &ShareId,
        folder_id: &FolderId,
        email: &str,
        role: &ShareRole,
    ) -> anyhow::Result<()> {
        self.action_guard(PermissionAction::ShareFolder {
            share_id: share_id.clone(),
            folder_id: folder_id.clone(),
        })
        .await?;

        let request = self
            .create_invites_request(share_id, email, role, None, Some(folder_id.clone()))
            .await
            .context("Error creating invite to folder request")?;

        self.send_invite(share_id, request)
            .await
            .context("Error sending invite to folder request")?;

        Ok(())
    }
}
