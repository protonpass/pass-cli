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

use crate::common::CodeResponse;
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::DELETE;
use pass_domain::InviteId;

impl<C: PassClientContext> PassClient<C> {
    pub async fn reject_invite(&self, invite_id: &InviteId) -> Result<()> {
        self.action_guard(PermissionAction::RejectInvite).await?;

        let res = self
            .send(DELETE!("/pass/v1/invite/{invite_id}"))
            .await
            .context("Error sending reject invite request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}
