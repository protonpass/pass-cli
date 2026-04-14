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
use anyhow::{Context, Result};
use pass_domain::Invite;

#[derive(serde::Serialize)]
struct InviteList {
    invites: Vec<InviteEntry>,
}

#[derive(serde::Serialize)]
struct InviteEntry {
    invite_type: String,
    inviter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl InviteEntry {
    pub fn from(invite: Invite) -> Self {
        let (invite_type, name) = match invite.vault_data {
            Some(data) => ("vault".to_string(), Some(data.vault_data.name)),
            None => ("item".to_string(), None),
        };

        Self {
            invite_type,
            inviter: invite.inviter_email,
            name,
        }
    }
}

pub async fn run(client: PassClient, output: OutputFormat) -> Result<()> {
    let invites = client
        .list_user_invites()
        .await
        .context("Error listing invites")?;

    let invites: Vec<Invite> = invites.into_iter().map(|i| i.invite).collect();
    match output {
        OutputFormat::Human => {
            for invite in invites {
                match invite.vault_data {
                    Some(vault_data) => {
                        println!(
                            "- [{}]: Type=Vault | Vault={} | From {}",
                            invite.id, vault_data.vault_data.name, invite.inviter_email
                        );
                    }
                    // Item
                    None => {
                        println!(
                            "- [{}]: Type=Item | From {}",
                            invite.id, invite.inviter_email
                        );
                    }
                }
            }
        }
        OutputFormat::Json => {
            let mapped: Vec<InviteEntry> = invites.into_iter().map(InviteEntry::from).collect();
            let instance = InviteList { invites: mapped };
            let as_json =
                serde_json::to_string_pretty(&instance).context("Error serializing invite list")?;

            println!("{}", as_json);
        }
    }

    Ok(())
}
