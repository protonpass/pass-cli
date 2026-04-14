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

use crate::commands::{OutputFormat, Role};
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;
use pass_domain::{ItemId, ShareId};

pub mod list;
pub mod remove;
pub mod update;

#[derive(Subcommand)]
pub enum MemberCommands {
    #[command(about = "List item members")]
    List {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: String,
        #[arg(long, help = "ID of the item")]
        item_id: String,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Update an item member's role")]
    Update {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: String,
        #[arg(long, help = "Member share ID")]
        member_share_id: String,
        #[arg(long, help = "New role for the member")]
        role: Role,
    },
    #[command(about = "Remove an item member")]
    Remove {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: String,
        #[arg(long, help = "Member share ID")]
        member_share_id: String,
    },
}

pub async fn run(client: PassClient, subcommand: MemberCommands) -> Result<()> {
    match subcommand {
        MemberCommands::List {
            share_id,
            item_id,
            output,
        } => list::run(client, ShareId::new(share_id), ItemId::new(item_id), output).await,
        MemberCommands::Update {
            share_id,
            member_share_id,
            role,
        } => {
            update::run(
                client,
                ShareId::new(share_id),
                ShareId::new(member_share_id),
                role,
            )
            .await
        }
        MemberCommands::Remove {
            share_id,
            member_share_id,
        } => {
            remove::run(
                client,
                ShareId::new(share_id),
                ShareId::new(member_share_id),
            )
            .await
        }
    }
}
