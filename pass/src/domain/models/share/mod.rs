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

mod member;
mod role;

pub use member::*;
pub use role::*;

use crate::domain::models::group::GroupId;
use crate::domain::{AddressId, FolderId, ItemId, VaultId};
use anyhow::{Result, anyhow};
use pass_derive::sdk_export;

// `ShareId` comes from the upstream `proton-pass-types` crate (see `item::mod` for the serde
// feature notes). Kept as a re-export here so existing `models::share::ShareId` paths keep working.
pub use proton_pass_types::ShareId;

#[derive(Clone, Debug)]
pub enum ShareType {
    Vault {
        vault_id: VaultId,
    },
    Item {
        vault_id: VaultId,
        item_id: ItemId,
    },
    Folder {
        vault_id: VaultId,
        folder_id: FolderId,
    },
}

#[sdk_export]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum TargetType {
    Vault,
    Item,
    Folder,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl TargetType {
    pub fn value(&self) -> u8 {
        match self {
            TargetType::Vault => 1,
            TargetType::Item => 2,
            TargetType::Folder => 3,
        }
    }

    pub fn from_value(value: u8) -> Result<Self> {
        match value {
            1 => Ok(TargetType::Vault),
            2 => Ok(TargetType::Item),
            3 => Ok(TargetType::Folder),
            _ => Err(anyhow!("Invalid target type: {value}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShareContent {
    pub content: Vec<u8>,
    pub share_key_rotation: u8,
    pub content_format_version: u8,
}

/// Bitmask flags attached to a share by the API (`Flags` field of the share response).
/// Mirrors the server-side `ShareFlag` enum; only the flags this client acts upon are modelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareFlag {
    /// The share is hidden from listings. Not acted upon by this client.
    Hidden,
    /// Sensitive content (the login password) of items in this share must not be displayed.
    HideSensitiveContent,
}

impl ShareFlag {
    fn bit(self) -> u64 {
        match self {
            ShareFlag::Hidden => 1 << 0,
            ShareFlag::HideSensitiveContent => 1 << 1,
        }
    }

    /// Parse a u64 bitmask value into a `Vec<ShareFlag>`
    pub fn parse_flags(flags: u64) -> Vec<ShareFlag> {
        let mut result = Vec::new();

        if flags & ShareFlag::Hidden.bit() != 0 {
            result.push(ShareFlag::Hidden);
        }
        if flags & ShareFlag::HideSensitiveContent.bit() != 0 {
            result.push(ShareFlag::HideSensitiveContent);
        }

        result
    }

    /// Convert a `Vec<ShareFlag>` back to a u64 bitmask
    pub fn to_bitmask(flags: &[ShareFlag]) -> u64 {
        flags.iter().fold(0u64, |acc, flag| acc | flag.bit())
    }
}

#[derive(Clone, Debug)]
pub struct Share {
    pub id: ShareId,
    pub address_id: AddressId,
    pub share_type: ShareType,
    pub vault_id: VaultId,
    pub permission: Permission,
    pub content: Option<ShareContent>,
    pub share_role: ShareRole,
    pub group_id: Option<GroupId>,
    pub flags: Vec<ShareFlag>,
}

impl Share {
    pub fn is_vault_share(&self) -> bool {
        matches!(self.share_type, ShareType::Vault { .. })
    }

    /// Whether the sensitive content (the login password) of items in this share must be kept
    /// hidden from the user.
    pub fn should_hide_sensitive_content(&self) -> bool {
        self.flags.contains(&ShareFlag::HideSensitiveContent)
    }

    pub fn vault_share_guard(&self) -> Result<()> {
        if self.is_vault_share() {
            Ok(())
        } else {
            Err(anyhow!("This operation is only valid for vault shares"))
        }
    }

    pub fn is_owner(&self) -> bool {
        matches!(self.share_role, ShareRole::Owner)
    }

    pub fn owner_guard(&self) -> Result<()> {
        if self.is_owner() {
            Ok(())
        } else {
            Err(anyhow!("This operation is only valid for owners"))
        }
    }

    pub fn is_item_share(&self) -> bool {
        matches!(self.share_type, ShareType::Item { .. })
    }

    pub fn is_folder_share(&self) -> bool {
        matches!(self.share_type, ShareType::Folder { .. })
    }

    pub fn is_group_share(&self) -> bool {
        self.group_id.is_some()
    }

    pub fn can_share(&self) -> bool {
        self.has_permission(PermissionFlag::Admin)
    }

    pub fn has_permission(&self, permission: PermissionFlag) -> bool {
        self.permission.has_flag(permission)
    }

    pub fn permission_guard(&self, permission: PermissionFlag) -> Result<()> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(anyhow!(
                "Share {} does not have the necessary permissions",
                self.id
            ))
        }
    }

    pub fn can_share_guard(&self) -> Result<()> {
        if self.can_share() {
            Ok(())
        } else {
            Err(anyhow!(
                "Share {} does not have sharing permissions",
                self.id
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flags_individual() {
        assert_eq!(ShareFlag::parse_flags(0), vec![]);
        assert_eq!(ShareFlag::parse_flags(1), vec![ShareFlag::Hidden]);
        assert_eq!(
            ShareFlag::parse_flags(2),
            vec![ShareFlag::HideSensitiveContent]
        );
    }

    #[test]
    fn parse_flags_combined_and_unknown_bits() {
        assert_eq!(
            ShareFlag::parse_flags(3),
            vec![ShareFlag::Hidden, ShareFlag::HideSensitiveContent]
        );
        // Unknown bits are ignored
        assert_eq!(
            ShareFlag::parse_flags(0b0110),
            vec![ShareFlag::HideSensitiveContent]
        );
    }

    #[test]
    fn to_bitmask_roundtrip() {
        assert_eq!(ShareFlag::to_bitmask(&[]), 0);
        assert_eq!(ShareFlag::to_bitmask(&[ShareFlag::Hidden]), 1);
        assert_eq!(
            ShareFlag::to_bitmask(&[ShareFlag::Hidden, ShareFlag::HideSensitiveContent]),
            3
        );
    }

    fn share_with_flags(flags: Vec<ShareFlag>) -> Share {
        Share {
            id: ShareId::new("share-id".to_string()),
            address_id: AddressId::new("address-id".to_string()),
            share_type: ShareType::Vault {
                vault_id: VaultId::new("vault-id".to_string()),
            },
            vault_id: VaultId::new("vault-id".to_string()),
            permission: Permission::default(),
            content: None,
            share_role: ShareRole::Owner,
            group_id: None,
            flags,
        }
    }

    #[test]
    fn should_hide_sensitive_content() {
        assert!(!share_with_flags(vec![]).should_hide_sensitive_content());
        assert!(!share_with_flags(vec![ShareFlag::Hidden]).should_hide_sensitive_content());
        assert!(
            share_with_flags(vec![ShareFlag::HideSensitiveContent]).should_hide_sensitive_content()
        );
        assert!(
            share_with_flags(vec![ShareFlag::Hidden, ShareFlag::HideSensitiveContent])
                .should_hide_sensitive_content()
        );
    }
}
