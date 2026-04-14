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

mod generate;
mod score;

use crate::commands::OutputFormat;
use anyhow::Result;
use clap::{Subcommand, ValueEnum};

#[derive(ValueEnum, Clone, Debug)]
pub enum PasswordFormat {
    Random,
    Memorable,
}

#[derive(Subcommand)]
pub enum PasswordCommands {
    #[command(about = "Generate a password")]
    Generate {
        #[command(subcommand)]
        command: generate::GeneratePasswordCommand,
    },
    #[command(about = "Score a password")]
    Score {
        #[arg(help = "Password to score")]
        password: String,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(command: &PasswordCommands) -> Result<()> {
    match command {
        PasswordCommands::Generate { command } => generate::run(command).await,
        PasswordCommands::Score { password, output } => score::score(password, output),
    }
}
