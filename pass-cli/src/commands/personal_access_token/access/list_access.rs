use super::super::PersonalAccessTokenQuery;
use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use pass::PersonalAccessTokenAccess;

pub async fn run(
    client: PassClient,
    query: PersonalAccessTokenQuery,
    output: Option<OutputFormat>,
) -> Result<()> {
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let personal_access_token_id = query.resolve(&client).await?;

    let access_list = client
        .list_personal_access_token_access(&personal_access_token_id)
        .await
        .context("Failed to list personal access token access")?;

    match output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&access_list)
                .context("Failed to serialize access list to JSON")?;
            println!("{}", json);
        }
        OutputFormat::Human => {
            if access_list.is_empty() {
                println!("No access grants found for this personal access token");
            } else {
                println!("Personal access token access grants:");
                println!();
                let current_tz = TimeZone::system();
                for access in access_list {
                    match access {
                        PersonalAccessTokenAccess::Vault {
                            share_id,
                            role,
                            expire_time,
                            vault_name,
                        } => {
                            let mut msg =
                                format!("- [{share_id}] {vault_name} | Type=Vault | Role={role}");

                            if let Some(expire_time) = expire_time {
                                let expires = format_timestamp(&current_tz, expire_time);
                                msg.push_str(&format!(" | Expires: {expires}"));
                            }
                            println!("{msg}");
                        }
                        PersonalAccessTokenAccess::Item {
                            share_id,
                            role,
                            expire_time,
                            item_title,
                            item_id: _,
                        } => {
                            let mut msg =
                                format!("- [{share_id}] {item_title} | Type=Item | Role={role}");

                            if let Some(expire_time) = expire_time {
                                let expires = format_timestamp(&current_tz, expire_time);
                                msg.push_str(&format!(" | Expires: {expires}"));
                            }
                            println!("{msg}");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn format_timestamp(tz: &TimeZone, timestamp: i64) -> String {
    let timestamp = Timestamp::from_second(timestamp).unwrap_or_default();
    let zoned = timestamp.to_zoned(tz.clone());
    let info = tz.to_offset_info(timestamp);

    format!(
        "{}-{:0>2}-{:0>2} {:0>2}:{:0>2} ({})",
        zoned.year(),
        zoned.month(),
        zoned.day(),
        zoned.hour(),
        zoned.minute(),
        info.abbreviation()
    )
}
