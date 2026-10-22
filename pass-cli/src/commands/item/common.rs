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

use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow, bail};
use pass::domain::{ItemId, Share, ShareId};
use std::ops::ControlFlow;

pub enum ShareQuery {
    ShareId(ShareId),
    VaultName(String),
}

impl ShareQuery {
    pub fn new(share_id: Option<String>, vault_name: Option<String>) -> Result<Self> {
        Self::new_with_arg_name(share_id, vault_name, "--share-id", "--vault-name")
    }

    pub fn new_with_arg_name(
        share_id: Option<String>,
        vault_name: Option<String>,
        share_id_arg: &str,
        vault_name_arg: &str,
    ) -> Result<Self> {
        match (share_id, vault_name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),
            (None, None) => Err(anyhow!(
                "Please provide either {share_id_arg} or {vault_name_arg}"
            )),
            (Some(_), Some(_)) => Err(anyhow!(
                "Please provide either {share_id_arg} or {vault_name_arg}, not both"
            )),
        }
    }

    pub async fn share_id(&self, client: &PassClient) -> Result<ShareId> {
        match self {
            ShareQuery::ShareId(share_id) => Ok(share_id.clone()),
            ShareQuery::VaultName(name) => match client.find_vault(name).await {
                Ok(v) => Ok(v.share_id),
                Err(e) => Err(anyhow!("Error finding vault [{name}]: {e}")),
            },
        }
    }
}

pub enum ItemQuery {
    ItemId(ItemId),
    ItemTitle(String),
}

impl ItemQuery {
    pub fn new(item_id: Option<String>, item_title: Option<String>) -> Result<Self> {
        Self::new_with_arg_name(item_id, item_title, "--item-id", "--item-title")
    }

    pub fn new_with_arg_name(
        item_id: Option<String>,
        item_title: Option<String>,
        item_id_arg: &str,
        item_title_arg: &str,
    ) -> Result<Self> {
        match (item_id, item_title) {
            (Some(item_id), None) => Ok(Self::ItemId(ItemId::new(item_id))),
            (None, Some(item_title)) => Ok(Self::ItemTitle(item_title)),
            (None, None) => Err(anyhow!(
                "Please provide either {item_id_arg} or {item_title_arg}"
            )),
            (Some(_), Some(_)) => Err(anyhow!(
                "Please provide either {item_id_arg} or {item_title_arg}, not both"
            )),
        }
    }

    pub async fn item_id(&self, share_id: &ShareId, client: &PassClient) -> Result<ItemId> {
        match self {
            Self::ItemId(id) => Ok(id.clone()),
            Self::ItemTitle(title) => {
                let matching_id = client
                    .list_items_until(share_id, |item| {
                        if item.content.title.eq(title) {
                            ControlFlow::Break(item.id.clone())
                        } else {
                            ControlFlow::Continue(())
                        }
                    })
                    .await
                    .context("Error listing items")?
                    .ok_or_else(|| anyhow!("No item found with title: {}", title))?;

                Ok(matching_id)
            }
        }
    }
}

pub const SENSITIVE_CONTENT_POLICY_ERROR: &str =
    "Viewing the contents of this item is not allowed by policy";

pub fn ensure_share_allows_viewing(share: &Share) -> Result<()> {
    if share.should_hide_sensitive_content() {
        bail!(SENSITIVE_CONTENT_POLICY_ERROR);
    }
    Ok(())
}

pub async fn ensure_sensitive_content_visible(
    client: &PassClient,
    share_id: &ShareId,
) -> Result<()> {
    let share = client
        .get_share(share_id)
        .await
        .with_context(|| format!("Error retrieving share {share_id}"))?;
    ensure_share_allows_viewing(&share)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pass::domain::{AddressId, Permission, ShareFlag, ShareRole, ShareType, VaultId};

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
    fn share_without_flags_allows_viewing() {
        assert!(ensure_share_allows_viewing(&share_with_flags(vec![])).is_ok());
        assert!(ensure_share_allows_viewing(&share_with_flags(vec![ShareFlag::Hidden])).is_ok());
    }

    #[test]
    fn share_with_hide_sensitive_content_flag_is_rejected() {
        let err =
            ensure_share_allows_viewing(&share_with_flags(vec![ShareFlag::HideSensitiveContent]))
                .expect_err("Should reject viewing the share contents");

        assert_eq!(SENSITIVE_CONTENT_POLICY_ERROR, err.to_string());
        assert!(err.to_string().contains("not allowed by policy"));
    }
}
