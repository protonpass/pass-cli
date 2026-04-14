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
use anyhow::{Context, Result};
use jiff::Timestamp;
use proton_pass_common::totp::TOTP;
use serde::Serialize;

#[derive(Serialize)]
struct TotpOutput {
    token: String,
}

pub async fn run(secret_or_uri: &str, output: &OutputFormat) -> Result<()> {
    if secret_or_uri.trim().is_empty() {
        return Err(anyhow::anyhow!("Empty secret or URI"));
    }
    // Create TOTP instance from URI or secret
    let totp = TOTP::from_uri(secret_or_uri)
        .context("Failed to parse TOTP URI or secret. Please provide a valid TOTP URI (otpauth://...) or base32 secret")?;

    // Get current timestamp in UTC
    let timestamp = Timestamp::now().as_second() as u64;

    // Generate token
    let token = totp
        .generate_token(timestamp)
        .context("Failed to generate TOTP token")?;

    // Output the token
    match output {
        OutputFormat::Human => {
            println!("{token}");
        }
        OutputFormat::Json => {
            let as_json = serde_json::to_string_pretty(&TotpOutput { token })
                .context("Error serializing TOTP token")?;
            println!("{as_json}");
        }
    }

    Ok(())
}
