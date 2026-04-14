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
use crate::utils::is_experimental_features_disabled;
use anyhow::Result;
use clap::Subcommand;
use pass_domain::InviteId;

pub mod accept;
mod group;
pub mod list;
pub mod reject;

#[derive(Subcommand)]
pub enum InviteCommands {
    #[command(about = "List pending invites")]
    List {
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Accept an invite")]
    Accept {
        #[arg(help = "ID of the invite to accept")]
        invite_id: String,
    },
    #[command(about = "Reject an invite")]
    Reject {
        #[arg(help = "ID of the invite to reject")]
        invite_id: String,
    },
    #[command(
        about = "Operations to perform on group invites",
        hide = is_experimental_features_disabled()
    )]
    Group {
        #[command(subcommand)]
        command: group::GroupInviteCommands,
    },
}

pub async fn run(subcommand: InviteCommands, client: PassClient) -> Result<()> {
    match subcommand {
        InviteCommands::List { output } => list::run(client, output).await,
        InviteCommands::Accept { invite_id } => accept::run(client, InviteId::new(invite_id)).await,
        InviteCommands::Reject { invite_id } => reject::run(client, InviteId::new(invite_id)).await,
        InviteCommands::Group { command } => group::run(command, client).await,
    }
}
