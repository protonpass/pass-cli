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
use anyhow::{Context, Result};
use muon::DELETE;
use pass_domain::{PersonalAccessTokenId, ShareId};

impl<C: PassClientContext> PassClient<C> {
    pub async fn revoke_personal_access_token_access(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
        share_id: &ShareId,
    ) -> Result<()> {
        self.personal_access_token_operation_guard()?;
        info!(
            "Revoking personal access token {personal_access_token_id} access from share {share_id}"
        );

        let res = self
            .send(DELETE!(
                "/pass/v1/personal-access-token/{}/access/{}",
                personal_access_token_id,
                share_id.value()
            ))
            .await
            .context("Failed to revoke personal access token access")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        info!(
            "Personal access token {} access revoked successfully from share {}",
            personal_access_token_id, share_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    #[muon_test::test]
    async fn test_revoke_access(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const PERSONAL_ACCESS_TOKEN_ID: &str = "test_sa_id";
        const SHARE_ID: &str = "test_share_id";
        const REVOKE_PATH: &str = "/pass/v1/personal-access-token/test_sa_id/access/test_share_id";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        let revoke_handled =
            api.handler_with_method(Method::DELETE, REVOKE_PATH, |_| success_code());

        client
            .revoke_personal_access_token_access(
                &PersonalAccessTokenId::new(PERSONAL_ACCESS_TOKEN_ID.to_string()),
                &ShareId::new(SHARE_ID.to_string()),
            )
            .await
            .expect("Should be able to revoke access");

        assert_hit!(revoke_handled);
    }
}
