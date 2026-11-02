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

use super::common::{ItemQuery, ShareQuery};
use crate::commands::item::agent_monitor::{ensure_reason_if_agent, send_reason_if_agent};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow, bail};
use pass::domain::{EventAction, FolderId, ShareId};

pub enum MoveTarget {
    // Move to another share
    Share(ShareQuery),
    // Move to a folder, optionally in another share
    Folder {
        folder_id: FolderId,
        share: Option<ShareQuery>,
    },
    // Move to the same share, but at the root level
    ShareRoot {
        share: Option<ShareQuery>,
    },
}

impl MoveTarget {
    pub fn new(
        to_share_id: Option<String>,
        to_vault_name: Option<String>,
        to_folder_id: Option<String>,
        to_share_root: bool,
    ) -> Result<Self> {
        let has_share = to_share_id.is_some() || to_vault_name.is_some();
        let share = || {
            if has_share {
                ShareQuery::new_with_arg_name(
                    to_share_id.clone(),
                    to_vault_name.clone(),
                    "--to-share-id",
                    "--to-vault-name",
                )
                .map(Some)
            } else {
                Ok(None)
            }
        };

        match (to_folder_id, to_share_root) {
            (Some(_), true) => {
                bail!("Please provide either --to-folder-id or --to-share-root, not both")
            }
            (Some(folder_id), false) => Ok(Self::Folder {
                folder_id: FolderId::new(folder_id),
                share: share()?,
            }),
            (None, true) => Ok(Self::ShareRoot { share: share()? }),
            (None, false) => match share()? {
                Some(share) => Ok(Self::Share(share)),
                None => bail!(
                    "Please provide a destination: --to-share-id, --to-vault-name, --to-folder-id or --to-share-root"
                ),
            },
        }
    }
}

fn ensure_same_share(from: &ShareId, to: &ShareId) -> Result<()> {
    if from == to {
        Ok(())
    } else {
        Err(anyhow!(
            "Moving an item to a folder or share root in a different vault is not supported"
        ))
    }
}

pub struct MoveItemQuery {
    from_share_query: ShareQuery,
    item_query: ItemQuery,
    target: MoveTarget,
}

impl MoveItemQuery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        from_share_id: Option<String>,
        from_vault: Option<String>,
        item_id: Option<String>,
        item_title: Option<String>,
        to_share_id: Option<String>,
        to_vault_name: Option<String>,
        to_folder_id: Option<String>,
        to_share_root: bool,
    ) -> Result<Self> {
        let from_share_query = ShareQuery::new_with_arg_name(
            from_share_id,
            from_vault,
            "--from-share-id",
            "--from-vault",
        )?;
        let item_query = ItemQuery::new(item_id, item_title)?;
        let target = MoveTarget::new(to_share_id, to_vault_name, to_folder_id, to_share_root)?;

        Ok(Self {
            from_share_query,
            item_query,
            target,
        })
    }
}

pub async fn run(client: PassClient, query: MoveItemQuery) -> Result<()> {
    ensure_reason_if_agent(&client)?;
    let from_share_id = query.from_share_query.share_id(&client).await?;
    let item_id = query.item_query.item_id(&from_share_id, &client).await?;

    let (folder_id, share) = match query.target {
        MoveTarget::Share(to_share_query) => {
            let to_share_id = to_share_query.share_id(&client).await?;
            return move_to_share(&client, &from_share_id, &item_id, &to_share_id).await;
        }
        MoveTarget::Folder { folder_id, share } => (Some(folder_id), share),
        MoveTarget::ShareRoot { share } => (None, share),
    };

    if let Some(share) = share {
        let to_share_id = share.share_id(&client).await?;
        ensure_same_share(&from_share_id, &to_share_id)?;
    }

    client
        .move_item_to_folder(&from_share_id, &item_id, folder_id.as_ref())
        .await
        .context("Error moving item")?;

    send_reason_if_agent(
        &client,
        EventAction::ItemUpdate,
        &from_share_id,
        Some(&item_id),
    )
    .await?;

    println!("{item_id}");
    Ok(())
}

async fn move_to_share(
    client: &PassClient,
    from_share_id: &ShareId,
    item_id: &pass::domain::ItemId,
    to_share_id: &ShareId,
) -> Result<()> {
    let new_item_id = client
        .move_item(from_share_id, item_id, to_share_id)
        .await
        .context("Error moving item")?;

    send_reason_if_agent(
        client,
        EventAction::ItemSoftDelete,
        from_share_id,
        Some(item_id),
    )
    .await?;

    send_reason_if_agent(
        client,
        EventAction::ItemCreate,
        to_share_id,
        Some(&new_item_id),
    )
    .await?;

    println!("{new_item_id}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn share_target() {
        assert!(matches!(
            MoveTarget::new(s("a"), None, None, false),
            Ok(MoveTarget::Share(_))
        ));
        assert!(matches!(
            MoveTarget::new(None, s("v"), None, false),
            Ok(MoveTarget::Share(_))
        ));
    }

    #[test]
    fn folder_target() {
        assert!(matches!(
            MoveTarget::new(None, None, s("f"), false),
            Ok(MoveTarget::Folder { share: None, .. })
        ));
        assert!(matches!(
            MoveTarget::new(s("a"), None, s("f"), false),
            Ok(MoveTarget::Folder { share: Some(_), .. })
        ));
    }

    #[test]
    fn root_target() {
        assert!(matches!(
            MoveTarget::new(None, None, None, true),
            Ok(MoveTarget::ShareRoot { share: None })
        ));
    }

    #[test]
    fn rejects_conflicts_and_empty() {
        assert!(MoveTarget::new(None, None, None, false).is_err());
        assert!(MoveTarget::new(None, None, s("f"), true).is_err());
        assert!(MoveTarget::new(s("a"), s("v"), None, false).is_err());
        assert!(MoveTarget::new(s("a"), s("v"), s("f"), false).is_err());
    }

    #[test]
    fn same_share_check() {
        let a = ShareId::new("a".to_string());
        let b = ShareId::new("b".to_string());
        assert!(ensure_same_share(&a, &a).is_ok());
        assert!(ensure_same_share(&a, &b).is_err());
    }
}
