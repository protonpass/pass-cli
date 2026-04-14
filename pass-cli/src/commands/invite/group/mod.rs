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

mod accept;
mod list;

use crate::commands::OutputFormat;
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;
use pass_domain::InviteId;

#[derive(Subcommand)]
pub enum GroupInviteCommands {
    #[command(about = "List pending invites")]
    List {
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Accept group invite")]
    Accept { invite_id: String },
}

pub async fn run(command: GroupInviteCommands, client: PassClient) -> Result<()> {
    match command {
        GroupInviteCommands::List { output } => list::run(client, output).await,
        GroupInviteCommands::Accept { invite_id } => {
            accept::run(client, InviteId::new(invite_id)).await
        }
    }
}
