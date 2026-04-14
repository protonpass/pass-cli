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

use crate::commands::OutputFormat;
use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum TotpCommands {
    #[command(about = "Generate a TOTP token from a secret or URI")]
    Generate {
        #[arg(help = "TOTP secret (base32) or URI (otpauth://...)")]
        secret_or_uri: String,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
}

pub async fn run(command: &TotpCommands) -> Result<()> {
    match command {
        TotpCommands::Generate {
            secret_or_uri,
            output,
        } => generate::run(secret_or_uri, output).await,
    }
}
