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
