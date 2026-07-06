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

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    #[serde(rename = "PasswordPolicy")]
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
