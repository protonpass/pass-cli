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
use crate::utils::debug_response;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::DELETE;
use pass_domain::ShareId;

impl<C: PassClientContext> PassClient<C> {
    pub async fn delete_vault(&self, share_id: &ShareId) -> Result<()> {
        self.action_guard(PermissionAction::DeleteVault {
            share_id: share_id.clone(),
        })
        .await?;
        let res = self
            .send(DELETE!("/pass/v1/vault/{}", share_id))
            .await
            .context("Failed to send delete Vault request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Error in delete Vault request: {}", res.status()));
        }

        self.clear_shares_cache().await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    #[muon_test::test]
    async fn test_delete_vault(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "MyShareID";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_vault_share(&api, SHARE_ID);
        let handled =
            api.handler_with_method(Method::DELETE, format!("/pass/v1/vault/{SHARE_ID}"), |_| {
                success(Empty)
            });

        client
            .delete_vault(&ShareId::new(SHARE_ID.to_string()))
            .await
            .expect("Should have been able to delete the vault");

        assert_hit!(handled);
    }
}
