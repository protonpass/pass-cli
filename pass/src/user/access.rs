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

use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::GET;

// Example enum for PlanType
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum PlanType {
    #[serde(rename = "free")]
    Free,
    #[serde(rename = "plus")]
    Plus,
    #[serde(rename = "business")]
    Business,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PassPlan {
    /// Type of plan for this user, can be free, plus or business
    #[serde(rename = "Type")]
    pub type_: PlanType,
    /// Internal name of the plan
    #[serde(rename = "InternalName")]
    pub internal_name: String,
    /// Display name of the plan
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    /// Whether this user can manage the plan
    #[serde(rename = "ManageSubscription")]
    pub manage_subscription: bool,
    /// If this user has a paid plan when does the subscription end
    #[serde(rename = "SubscriptionEnd")]
    pub subscription_end: Option<u64>,
    /// If this user has a plaid plan, and the plan is set to auto-renew
    #[serde(rename = "SubscriptionRenewal")]
    pub subscription_renewal: bool,
    /// Coupon used for this subscription if any was used
    #[serde(rename = "SubscriptionCoupon")]
    pub subscription_coupon: Option<String>,
    /// If the user used an offer, what offer was that
    #[serde(rename = "SubscriptionOffer")]
    pub subscription_offer: Option<String>,
    /// Force hide the upgrade button independently of the plan
    #[serde(rename = "HideUpgrade")]
    pub hide_upgrade: bool,
    /// If the user is in trial, show when the trial ends. Otherwise, it will be null
    #[serde(rename = "TrialEnd")]
    pub trial_end: Option<u64>,
    /// Vault limit, null for plans with Pass plus
    #[serde(rename = "VaultLimit")]
    pub vault_limit: Option<u16>,
    /// Alias limit, null for plans with Pass plus
    #[serde(rename = "AliasLimit")]
    pub alias_limit: Option<u16>,
    /// TOTP limit, null for plans with Pass plus
    #[serde(rename = "TotpLimit")]
    pub totp_limit: Option<u16>,
    /// Whether this account can manage alias configuration
    #[serde(rename = "ManageAlias")]
    pub manage_alias: bool,
    /// Whether this account can use file attachments
    #[serde(rename = "StorageAllowed")]
    pub storage_allowed: bool,
    /// Max allowed upload size in bytes of files
    #[serde(rename = "StorageMaxFileSize")]
    pub storage_max_file_size: u64,
    /// What is the storage usage for this user
    #[serde(rename = "StorageUsed")]
    pub storage_used: u64,
    /// What is the storage quota for this user
    #[serde(rename = "StorageQuota")]
    pub storage_quota: u64,
    /// Can use CLI flag
    #[serde(rename = "CliAllowed")]
    pub cli_allowed: Option<bool>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct MonitorStatus {
    /// If the monitor for proton address leaks is enabled
    #[serde(rename = "ProtonAddress")]
    pub proton_address: bool,
    /// If the monitor for aliases leaks is enabled
    #[serde(rename = "Aliases")]
    pub aliases: bool,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct UserDataSettings {
    /// Default share to user for this user. Null if not set any default share
    #[serde(rename = "DefaultShareID")]
    pub default_share_id: Option<String>,
    /// Alias sync enabled
    #[serde(rename = "AliasSyncEnabled")]
    pub alias_sync_enabled: bool,
    /// How many aliases are waiting to be synced
    #[serde(rename = "PendingAliasToSync")]
    pub pending_alias_to_sync: u16,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct UserInfo {
    #[serde(rename = "Plan")]
    pub plan: PassPlan,
    #[serde(rename = "Monitor")]
    pub monitor: MonitorStatus,
    #[serde(rename = "PendingInvites")]
    pub pending_invites: u16,
    #[serde(rename = "WaitingNewUserInvites")]
    pub waiting_new_user_invites: u16,
    #[serde(rename = "UserData")]
    pub user_data: UserDataSettings,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GetUserInfoResponse {
    #[serde(rename = "Access")]
    pub access: UserInfo,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn get_user_access(&self) -> Result<UserInfo> {
        let res = self
            .send(GET!("/pass/v1/user/access"))
            .await
            .context("Error retrieving user info")?;

        let response: GetUserInfoResponse = assert_response!(res);
        Ok(response.access)
    }

    pub async fn can_use_cli(&self) -> Result<bool> {
        let ff = self
            .has_feature_flag(pass_domain::FeatureFlag::PassCanUseCli)
            .await
            .context("Error checking PassCanUseCli feature flag")?;
        if !ff {
            return Ok(false);
        }

        let info = self
            .get_user_access()
            .await
            .context("Error retrieving user access info")?;

        let plan = info.plan;
        debug!("Checking is_login_allowed with plan {:?}", plan);
        match plan.cli_allowed {
            Some(v) => Ok(v),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::ProtonApiErrorCode;
    use crate::test_tools::*;

    #[muon_test::test]
    async fn test_get_user_access_returns_session_locked_error(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;

        // Set up handler to return a 400 with SessionLocked error code
        api.handler("/pass/v1/user/access", |_| {
            error_response_from_json(
                400,
                &format!("{{\"Code\":{}}}", ProtonApiErrorCode::SessionLocked as i32),
            )
        });

        let result = client.get_user_access().await;

        assert!(result.is_err(), "Should return an error for session locked");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("SessionLocked"),
            "Error should mention SessionLocked, got: {err}"
        );
    }

    #[muon_test::test]
    async fn test_get_user_access_returns_other_proton_error(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        let client = make_test_pass_client(raw_client, &api).await;

        // Set up handler to return a 400 with a different error code
        api.handler("/pass/v1/user/access", |_| {
            error_response_from_json(
                400,
                &format!("{{\"Code\":{}}}", ProtonApiErrorCode::AlreadyExists as i32),
            )
        });

        let result = client.get_user_access().await;

        assert!(result.is_err(), "Should return an error");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("AlreadyExists"),
            "Error should mention AlreadyExists, got: {err}"
        );
    }
}
