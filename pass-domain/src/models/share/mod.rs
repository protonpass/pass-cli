mod member;
mod role;

pub use member::*;
pub use role::*;

use crate::{AddressId, ItemId, VaultId};
use anyhow::{Result, anyhow};

#[derive(Clone, Debug, Hash, Eq, PartialEq, serde::Serialize)]
pub struct ShareId(pub(crate) String);

impl ShareId {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

display_for_basic!(ShareId);

#[derive(Clone, Debug)]
pub enum ShareType {
    Vault { vault_id: VaultId },
    Item { vault_id: VaultId, item_id: ItemId },
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum TargetType {
    Vault,
    Item,
}

impl TargetType {
    pub fn value(&self) -> u8 {
        match self {
            TargetType::Vault => 1,
            TargetType::Item => 2,
        }
    }

    pub fn from_value(value: u8) -> Result<Self> {
        match value {
            1 => Ok(TargetType::Vault),
            2 => Ok(TargetType::Item),
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

#[derive(Clone, Debug)]
pub struct Share {
    pub id: ShareId,
    pub address_id: AddressId,
    pub share_type: ShareType,
    pub vault_id: VaultId,
    pub permission: Permission,
    pub content: Option<ShareContent>,
}

impl Share {
    pub fn is_vault_share(&self) -> bool {
        matches!(self.share_type, ShareType::Vault { .. })
    }

    pub fn is_item_share(&self) -> bool {
        matches!(self.share_type, ShareType::Item { .. })
    }

    pub fn can_share(&self) -> bool {
        self.permission.has_flag(PermissionFlag::Admin)
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
