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

use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use jiff::{Timestamp, tz::TimeZone};

pub async fn run(client: PassClient, output: Option<OutputFormat>) -> Result<()> {
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let personal_access_tokens = client
        .list_personal_access_tokens()
        .await
        .context("Failed to list personal access tokens")?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&personal_access_tokens)
                .context("Error serializing personal access tokens")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            if personal_access_tokens.is_empty() {
                println!("No personal access tokens found");
            } else {
                for pat in personal_access_tokens {
                    let expiration = match pat.expire_time {
                        Some(ts) => format!(" (expires: {})", format_date(ts)),
                        None => String::new(),
                    };
                    println!("- [{}]: {}{}", pat.pat_id, pat.name, expiration);
                }
            }
        }
    }

    Ok(())
}

fn format_date(timestamp: i64) -> String {
    let ts = match Timestamp::from_second(timestamp) {
        Ok(ts) => ts,
        Err(_) => return format!("invalid ({})", timestamp),
    };
    let zoned = ts.to_zoned(TimeZone::UTC);
    format!("{}-{:02}-{:02}", zoned.year(), zoned.month(), zoned.day())
}
