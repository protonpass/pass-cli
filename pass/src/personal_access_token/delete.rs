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
use crate::{PassClient, PassClientContext};
use anyhow::Context;
use muon::DELETE;
use pass_domain::PersonalAccessTokenId;

impl<C: PassClientContext> PassClient<C> {
    pub async fn delete_personal_access_token(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
    ) -> anyhow::Result<()> {
        self.personal_access_token_operation_guard()?;
        info!("Deleting personal access token: {personal_access_token_id}");

        let res = self
            .send(DELETE!(
                "/account/v4/personal-access-token/{personal_access_token_id}"
            ))
            .await
            .context("Failed to delete personal access token")?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}
