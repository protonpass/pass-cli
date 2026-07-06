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

use crate::{PassClient, PassClientContext, PlanType};
use anyhow::{Context, Result};
use muon::GET;

// Re-export types from pass-domain to maintain backwards compatibility
pub use pass_domain::{
    OrganizationAliasCreateMode, OrganizationExportMode, OrganizationInfo,
    OrganizationPasswordPolicy, OrganizationSettings, OrganizationShareMode,
    OrganizationVaultCreateMode,
};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct GetOrganizationResponse {
    #[serde(rename = "Organization")]
    organization: OrganizationInfo,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn get_organization_policy(&self) -> Result<Option<OrganizationInfo>> {
        if let Some(cached) = self.get_cached_organization_policy().await? {
            return Ok(Some(cached));
        }

        let plan_type = self
            .get_user_access()
            .await
            .context("Error getting user access data")?
            .plan
            .plan_type;
        self.refresh_organization_policy(plan_type).await
    }

    pub(crate) async fn get_organization_policy_for_plan(
        &self,
        plan_type: PlanType,
    ) -> Result<Option<OrganizationInfo>> {
        if plan_type != PlanType::Business {
            return Ok(None);
        }

        if let Some(cached) = self.get_cached_organization_policy().await? {
            return Ok(Some(cached));
        }

        self.refresh_organization_policy(plan_type).await
    }

    async fn get_cached_organization_policy(&self) -> Result<Option<OrganizationInfo>> {
        let storage = self
            .client_features
            .get_data_storage()
            .await?
            .get_organization_policy_storage()
            .await;

        let Some(entry) = storage.get_policy().await? else {
            return Ok(None);
        };

        // Cache expiration is now handled by the storage implementation
        Ok(Some(entry.policy))
    }

    async fn refresh_organization_policy(
        &self,
        plan_type: PlanType,
    ) -> Result<Option<OrganizationInfo>> {
        if plan_type != PlanType::Business {
            return Ok(None);
        }

        let res = self
            .send(GET!("/pass/v1/organization"))
            .await
            .context("Error fetching organization policy")?;
        let response: GetOrganizationResponse = assert_response!(res);

        let storage = self
            .client_features
            .get_data_storage()
            .await?
            .get_organization_policy_storage()
            .await;
        storage.set_policy(&response.organization).await?;

        Ok(Some(response.organization))
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::test_tools::*;

    mod http {
        use super::*;

        #[muon_test::test]
        async fn non_business_plan_skips_fetch(server: muon_test::Server) {
            let (raw_client, api) = server.client::<()>();
            let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Plus).await;

            let handled = api.handler("/pass/v1/organization", |_| {
                success(serde_json::json!({ "Code": 1000 }))
            });

            let policy = client.get_organization_policy().await.unwrap();

            assert!(policy.is_none());
            assert_not_hit!(handled);
        }

        #[muon_test::test]
        async fn business_plan_fetches_and_caches(server: muon_test::Server) {
            let (raw_client, api) = server.client::<()>();
            let client =
                make_test_pass_client_with_setup(raw_client, &api, PlanType::Business).await;

            let handled = api.handler("/pass/v1/organization", |_| {
                success(serde_json::json!({
                    "Code": 1000,
                    "Organization": {
                        "CanUpdate": true,
                        "Settings": {
                            "ShareMode": 0,
                            "ShareAcceptMode": 0,
                            "ItemShareMode": 0,
                            "PublicLinkMode": 0,
                            "ForceLockSeconds": 0,
                            "ExportMode": 0,
                            "PasswordPolicy": {
                                "RandomPasswordAllowed": true,
                                "RandomPasswordMinLength": 4,
                                "RandomPasswordMaxLength": 64,
                                "RandomPasswordMustIncludeNumbers": true,
                                "RandomPasswordMustIncludeSymbols": true,
                                "RandomPasswordMustIncludeUppercase": true,
                                "MemorablePasswordAllowed": true,
                                "MemorablePasswordMinWords": 4,
                                "MemorablePasswordMaxWords": 8,
                                "MemorablePasswordMustCapitalize": true,
                                "MemorablePasswordMustIncludeNumbers": true
                            },
                            "VaultCreateMode": 1,
                            "AliasCreateMode": 1
                        }
                    }
                }))
            });

            let policy = client
                .get_organization_policy()
                .await
                .unwrap()
                .expect("Business plan should return an organization policy");

            assert_hit!(handled);
            assert!(policy.can_update);
            assert_eq!(
                OrganizationVaultCreateMode::OnlyOrgAdmins,
                policy.settings.vault_create_mode
            );
            assert_eq!(
                OrganizationAliasCreateMode::Nobody,
                policy.settings.alias_create_mode
            );
        }
    }
}
