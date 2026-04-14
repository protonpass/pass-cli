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

mod list;

use crate::commands::OutputFormat;
use crate::commands::share::list::ShareListMode;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ShareCommands {
    #[command(about = "List available shares")]
    List {
        #[arg(long, help = "Only display item shares", default_value = "false")]
        only_items: Option<bool>,
        #[arg(long, help = "Only display vault shares", default_value = "false")]
        only_vaults: Option<bool>,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(command: ShareCommands, client: PassClient) -> Result<()> {
    match command {
        ShareCommands::List {
            only_items,
            only_vaults,
            output,
        } => {
            let mode =
                ShareListMode::from_args(only_vaults.unwrap_or(false), only_items.unwrap_or(false))
                    .context("Error parsing arguments")?;
            list::run(client, mode, output).await
        }
    }
}
