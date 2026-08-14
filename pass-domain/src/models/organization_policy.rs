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

use serde::{Deserialize, Serialize};

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum OrganizationVaultCreateMode {
    Allowed,
    OnlyOrgAdmins,
    OnlyOrgAdminsAndPersonalVault,
}

impl TryFrom<u8> for OrganizationVaultCreateMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Allowed),
            1 => Ok(Self::OnlyOrgAdmins),
            2 => Ok(Self::OnlyOrgAdminsAndPersonalVault),
            other => Err(format!(
                "Invalid OrganizationVaultCreateMode value: {other}"
            )),
        }
    }
}

impl From<OrganizationVaultCreateMode> for u8 {
    fn from(value: OrganizationVaultCreateMode) -> Self {
        match value {
            OrganizationVaultCreateMode::Allowed => 0,
            OrganizationVaultCreateMode::OnlyOrgAdmins => 1,
            OrganizationVaultCreateMode::OnlyOrgAdminsAndPersonalVault => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum OrganizationAliasCreateMode {
    AllowedForAllMembers,
    Nobody,
}

impl TryFrom<u8> for OrganizationAliasCreateMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::AllowedForAllMembers),
            1 => Ok(Self::Nobody),
            other => Err(format!(
                "Invalid OrganizationAliasCreateMode value: {other}"
            )),
        }
    }
}

impl From<OrganizationAliasCreateMode> for u8 {
    fn from(value: OrganizationAliasCreateMode) -> Self {
        match value {
            OrganizationAliasCreateMode::AllowedForAllMembers => 0,
            OrganizationAliasCreateMode::Nobody => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum OrganizationExportMode {
    Unrestricted,
    OnlyAdmins,
}

impl TryFrom<u8> for OrganizationExportMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unrestricted),
            1 => Ok(Self::OnlyAdmins),
            other => Err(format!("Invalid OrganizationExportMode value: {other}")),
        }
    }
}

impl From<OrganizationExportMode> for u8 {
    fn from(value: OrganizationExportMode) -> Self {
        match value {
            OrganizationExportMode::Unrestricted => 0,
            OrganizationExportMode::OnlyAdmins => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum OrganizationShareMode {
    Unrestricted,
    RestrictedToOrganization,
}

impl TryFrom<u8> for OrganizationShareMode {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unrestricted),
            1 => Ok(Self::RestrictedToOrganization),
            other => Err(format!("Invalid OrganizationShareMode value: {other}")),
        }
    }
}

impl From<OrganizationShareMode> for u8 {
    fn from(value: OrganizationShareMode) -> Self {
        match value {
            OrganizationShareMode::Unrestricted => 0,
            OrganizationShareMode::RestrictedToOrganization => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct OrganizationPasswordPolicy {
    #[serde(rename = "RandomPasswordAllowed")]
    pub random_password_allowed: bool,
    #[serde(rename = "RandomPasswordMinLength")]
    pub random_password_min_length: Option<u32>,
    #[serde(rename = "RandomPasswordMaxLength")]
    pub random_password_max_length: Option<u32>,
    #[serde(rename = "RandomPasswordMustIncludeNumbers")]
    pub random_password_must_include_numbers: Option<bool>,
    #[serde(rename = "RandomPasswordMustIncludeSymbols")]
    pub random_password_must_include_symbols: Option<bool>,
    #[serde(rename = "RandomPasswordMustIncludeUppercase")]
    pub random_password_must_include_uppercase: Option<bool>,
    #[serde(rename = "MemorablePasswordAllowed")]
    pub memorable_password_allowed: bool,
    #[serde(rename = "MemorablePasswordMinWords")]
    pub memorable_password_min_words: Option<u32>,
    #[serde(rename = "MemorablePasswordMaxWords")]
    pub memorable_password_max_words: Option<u32>,
    #[serde(rename = "MemorablePasswordMustCapitalize")]
    pub memorable_password_must_capitalize: Option<bool>,
    #[serde(rename = "MemorablePasswordMustIncludeNumbers")]
    pub memorable_password_must_include_numbers: Option<bool>,
}

impl Default for OrganizationPasswordPolicy {
    fn default() -> Self {
        Self {
            random_password_allowed: true,
            random_password_min_length: None,
            random_password_max_length: None,
            random_password_must_include_numbers: None,
            random_password_must_include_symbols: None,
            random_password_must_include_uppercase: None,
            memorable_password_allowed: true,
            memorable_password_min_words: None,
            memorable_password_max_words: None,
            memorable_password_must_capitalize: None,
            memorable_password_must_include_numbers: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrganizationSettings {
    #[serde(rename = "ShareMode")]
    pub share_mode: OrganizationShareMode,
    #[serde(rename = "ShareAcceptMode")]
    pub share_accept_mode: OrganizationShareMode,
    #[serde(rename = "ItemShareMode")]
    pub item_share_mode: u8,
    #[serde(rename = "PublicLinkMode")]
    pub public_link_mode: u8,
    #[serde(rename = "ForceLockSeconds")]
    pub force_lock_seconds: u32,
    #[serde(rename = "ExportMode")]
    pub export_mode: OrganizationExportMode,
    #[serde(
        default,
        rename = "PasswordPolicy",
        deserialize_with = "deserialize_null_default"
    )]
    pub password_policy: OrganizationPasswordPolicy,
    #[serde(rename = "VaultCreateMode")]
    pub vault_create_mode: OrganizationVaultCreateMode,
    #[serde(rename = "AliasCreateMode")]
    pub alias_create_mode: OrganizationAliasCreateMode,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrganizationInfo {
    #[serde(rename = "CanUpdate")]
    pub can_update: bool,
    #[serde(rename = "Settings")]
    pub settings: OrganizationSettings,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn organization_info_with_null_password_policy_uses_default() {
        let json = serde_json::json!({
            "CanUpdate": true,
            "Settings": {
                "ShareMode": 0,
                "ShareAcceptMode": 0,
                "ItemShareMode": 1,
                "PublicLinkMode": 1,
                "ForceLockSeconds": 0,
                "ExportMode": 0,
                "PasswordPolicy": null,
                "VaultCreateMode": 0,
                "AliasCreateMode": 0
            }
        });

        let info: OrganizationInfo =
            serde_json::from_value(json).expect("null PasswordPolicy should not fail to parse");

        assert_eq!(
            info.settings.password_policy,
            OrganizationPasswordPolicy::default()
        );
    }

    #[test]
    fn organization_info_with_missing_password_policy_uses_default() {
        let json = serde_json::json!({
            "CanUpdate": true,
            "Settings": {
                "ShareMode": 0,
                "ShareAcceptMode": 0,
                "ItemShareMode": 1,
                "PublicLinkMode": 1,
                "ForceLockSeconds": 0,
                "ExportMode": 0,
                "VaultCreateMode": 0,
                "AliasCreateMode": 0
            }
        });

        let info: OrganizationInfo =
            serde_json::from_value(json).expect("missing PasswordPolicy should not fail to parse");

        assert_eq!(
            info.settings.password_policy,
            OrganizationPasswordPolicy::default()
        );
    }

    #[test]
    fn organization_info_with_present_password_policy_is_preserved() {
        let json = serde_json::json!({
            "CanUpdate": true,
            "Settings": {
                "ShareMode": 0,
                "ShareAcceptMode": 0,
                "ItemShareMode": 1,
                "PublicLinkMode": 1,
                "ForceLockSeconds": 0,
                "ExportMode": 0,
                "PasswordPolicy": {
                    "RandomPasswordAllowed": false,
                    "RandomPasswordMinLength": 4,
                    "RandomPasswordMaxLength": 64,
                    "RandomPasswordMustIncludeNumbers": true,
                    "RandomPasswordMustIncludeSymbols": true,
                    "RandomPasswordMustIncludeUppercase": true,
                    "MemorablePasswordAllowed": false,
                    "MemorablePasswordMinWords": 4,
                    "MemorablePasswordMaxWords": 8,
                    "MemorablePasswordMustCapitalize": true,
                    "MemorablePasswordMustIncludeNumbers": true
                },
                "VaultCreateMode": 0,
                "AliasCreateMode": 0
            }
        });

        let info: OrganizationInfo =
            serde_json::from_value(json).expect("present PasswordPolicy should parse");

        assert!(!info.settings.password_policy.random_password_allowed);
        assert!(!info.settings.password_policy.memorable_password_allowed);
    }
}
