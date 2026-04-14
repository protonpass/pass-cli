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
use anyhow::Result;
use clap::Subcommand;

pub mod set;
pub mod unset;
pub mod view;

#[derive(Subcommand)]
pub enum SettingsCommands {
    #[command(about = "View all current settings")]
    View,

    #[command(about = "Set a setting value", subcommand)]
    Set(set::SetCommands),

    #[command(about = "Unset (clear) a setting", subcommand)]
    Unset(unset::UnsetCommands),
}

pub async fn run(subcommand: SettingsCommands, client: PassClient) -> Result<()> {
    match subcommand {
        SettingsCommands::View => view::run(client).await,
        SettingsCommands::Set(cmd) => set::run(cmd, client).await,
        SettingsCommands::Unset(cmd) => unset::run(cmd, client).await,
    }
}
