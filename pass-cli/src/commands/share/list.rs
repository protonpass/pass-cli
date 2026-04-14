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

use crate::commands::OutputFormat;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::{Share, ShareId, ShareRole, ShareType, TargetType};

pub(crate) enum ShareListMode {
    OnlyItems,
    OnlyVaults,
    All,
}

#[derive(Debug, serde::Serialize)]
struct ShareEntry {
    id: ShareId,
    name: String,
    share_type: TargetType,
    share_role: ShareRole,
}

#[derive(Debug, serde::Serialize)]
struct SharesList {
    shares: Vec<ShareEntry>,
}

impl ShareListMode {
    pub fn from_args(only_vaults: bool, only_items: bool) -> Result<Self> {
        match (only_vaults, only_items) {
            (true, true) => Err(anyhow!(
                "You cannot pass only_vaults and only_items together"
            )),
            (true, false) => Ok(Self::OnlyVaults),
            (false, true) => Ok(Self::OnlyItems),
            (false, false) => Ok(Self::All),
        }
    }
}

pub async fn run(
    client: PassClient,
    share_list_mode: ShareListMode,
    output_format: OutputFormat,
) -> Result<()> {
    let shares = client.list_shares().await.context("Error listing shares")?;

    let shares_to_list = match share_list_mode {
        ShareListMode::OnlyItems => shares.into_iter().filter(|s| s.is_item_share()).collect(),
        ShareListMode::OnlyVaults => shares.into_iter().filter(|s| s.is_vault_share()).collect(),
        ShareListMode::All => shares,
    };

    let shares_to_print = adapt_shares(client, shares_to_list)
        .await
        .context("Error preparing shares for listing")?;

    match output_format {
        OutputFormat::Human => {
            for share in shares_to_print {
                println!(
                    "- [{}] Type={} | Role={} | {}",
                    share.id, share.share_type, share.share_role, share.name
                )
            }
        }
        OutputFormat::Json => {
            let as_str = serde_json::to_string_pretty(&SharesList {
                shares: shares_to_print,
            })
            .context("Error serializing output")?;
            println!("{as_str}");
        }
    }

    Ok(())
}

async fn adapt_shares(client: PassClient, shares: Vec<Share>) -> Result<Vec<ShareEntry>> {
    let mut res = Vec::with_capacity(shares.len());
    for share in shares {
        let (name, target_type) = match share.share_type {
            ShareType::Vault { .. } => {
                let content = client
                    .open_vault_share_content(&share.id, share.content)
                    .await;
                let name = match content {
                    Ok(content) => content.name,
                    Err(e) => {
                        error!("Error opening vault share content: {e:#}");
                        "ERROR".to_string()
                    }
                };

                (name, TargetType::Vault)
            }
            ShareType::Item { item_id, .. } => {
                let item = client
                    .view_item(&share.id, &item_id)
                    .await
                    .context("Error opening item")?;
                (item.item.content.title, TargetType::Item)
            }
        };

        res.push(ShareEntry {
            id: share.id,
            share_type: target_type,
            share_role: share.share_role,
            name,
        });
    }

    Ok(res)
}
