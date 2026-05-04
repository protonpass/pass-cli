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

use anyhow::Context;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use std::io::Write;
use std::path::PathBuf;

const PROTON_PASS_SESSION_DIR_ENV: &str = "PROTON_PASS_SESSION_DIR";
const PROTON_PASS_EXPERIMENTAL_FEATURES_ENV: &str = "PROTON_PASS_EXPERIMENTAL_FEATURES";

pub fn ask_for_input(prompt: &str, secure: bool) -> anyhow::Result<String> {
    if secure {
        let input = rpassword::prompt_password(prompt).context("Error prompting for password")?;
        Ok(input.replace("\n", "").trim().to_string())
    } else {
        let stdin = std::io::stdin();
        loop {
            let mut value = String::new();
            std::io::stdout()
                .write(prompt.as_bytes())
                .context("Error writing to stdout")?;
            std::io::stdout().flush().context("Error flushing stdout")?;

            stdin.read_line(&mut value)?;

            if !value.trim().is_empty() {
                return Ok(value.replace("\n", "").trim().to_string());
            } else {
                eprintln!("Value is empty");
            }
        }
    }
}

pub fn get_base_dir() -> anyhow::Result<PathBuf> {
    // Check for environment variable override first
    let proton_dir = if let Ok(custom_dir) = std::env::var(PROTON_PASS_SESSION_DIR_ENV) {
        PathBuf::from(custom_dir)
    } else {
        // Use platform-specific data directory
        let data_dir =
            dirs::data_dir().context("Failed to determine data directory for this platform")?;
        data_dir.join("proton-pass-cli")
    };

    // Create a .session subfolder (just like before, but in the platform-specific location)
    let session_dir = proton_dir.join(".session");

    // Create the directory if it doesn't exist, with owner-only permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&session_dir)
            .context("Error creating session directory")?;
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(&session_dir).context("Error creating session directory")?;
    }

    // Return the canonicalized (absolute) path
    let session_dir_absolute =
        std::fs::canonicalize(&session_dir).context("Error getting absolute path")?;
    Ok(session_dir_absolute)
}

pub fn is_experimental_features_disabled() -> bool {
    std::env::var(PROTON_PASS_EXPERIMENTAL_FEATURES_ENV)
        .map(|v| v != "on")
        .unwrap_or(true)
}

pub fn format_date(timestamp: i64) -> String {
    let ts = match Timestamp::from_second(timestamp) {
        Ok(ts) => ts,
        Err(_) => return format!("invalid ({})", timestamp),
    };
    let zoned = ts.to_zoned(TimeZone::UTC);
    format!("{}-{:02}-{:02}", zoned.year(), zoned.month(), zoned.day())
}

/// Format a timestamp (UTC) to current system timezine with time portion
#[allow(dead_code)]
pub fn format_timestamp_with_time(timestamp: Timestamp) -> String {
    let zoned = timestamp.to_zoned(TimeZone::system());
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        zoned.year(),
        zoned.month(),
        zoned.day(),
        zoned.hour(),
        zoned.minute(),
        zoned.second()
    )
}
