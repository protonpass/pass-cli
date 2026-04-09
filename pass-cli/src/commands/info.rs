use crate::commands::update::InstallSource;
use crate::commands::{OutputFormat, settings_helper, update};
use crate::helpers::CliPassClient as PassClient;
use crate::telemetry::event::CommandEvent;
use anyhow::{Context, Result};
use pass_domain::AccountType;
use std::path::PathBuf;

#[derive(serde::Serialize)]
struct InfoOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    pub release_track: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personal_access_token_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_source: Option<String>,
}

pub async fn run(
    client: PassClient,
    base_dir: PathBuf,
    output: Option<OutputFormat>,
) -> Result<()> {
    client.emit_telemetry(&CommandEvent::new("info")).await;

    // Resolve output format from settings if not provided
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let info_output = match client.account_type() {
        AccountType::User => {
            let info = client.get_info().await.context("Error getting user info")?;

            // Only show ENV if it's not "prod"
            let env_str = format!("{:?}", info.env);
            let env = if env_str != "Prod" {
                Some(env_str)
            } else {
                None
            };

            // Show release track
            let release_track = update::get_release_track(&base_dir)
                .await
                .unwrap_or_else(|_| "stable".to_string());

            let install_source = update::get_install_source()?;
            let install_source_str = if install_source != InstallSource::Standard {
                Some(format!("{:?}", install_source))
            } else {
                None
            };

            InfoOutput {
                env,
                release_track,
                id: info.user.id,
                username: Some(info.user.name),
                email: Some(info.user.email),
                personal_access_token_name: None,
                install_source: install_source_str,
            }
        }
        AccountType::PersonalAccessToken => {
            let personal_access_token_name = client
                .get_personal_access_token_name()
                .await
                .context("Error getting personal access token name")?;

            let env = None; // Personal access tokens might not have env info

            // Show release track
            let release_track = update::get_release_track(&base_dir)
                .await
                .unwrap_or_else(|_| "stable".to_string());

            let install_source = update::get_install_source()?;
            let install_source_str = if install_source != InstallSource::Standard {
                Some(format!("{:?}", install_source))
            } else {
                None
            };

            InfoOutput {
                env,
                release_track,
                id: "N/A".to_string(), // Personal access tokens don't have user IDs
                username: None,
                email: None,
                personal_access_token_name: Some(personal_access_token_name),
                install_source: install_source_str,
            }
        }
    };

    print(info_output, output).context("Error printing info")?;

    Ok(())
}

fn print(info: InfoOutput, output: OutputFormat) -> Result<()> {
    match output {
        OutputFormat::Human => {
            if let Some(env) = &info.env {
                println!("- ENV: {}", env);
            }
            println!("- Release track: {}", info.release_track);
            if let Some(personal_access_token_name) = &info.personal_access_token_name {
                println!("- Personal Access Token: {}", personal_access_token_name);
            } else {
                println!("- ID: {}", info.id);
                if let Some(username) = &info.username {
                    println!("- Username: {}", username);
                }
                if let Some(email) = &info.email {
                    println!("- Email: {}", email);
                }
            }
            if let Some(install_source) = &info.install_source {
                println!("- Install source: {}", install_source);
            }
        }
        OutputFormat::Json => {
            let as_json = serde_json::to_string_pretty(&info).context("Error serializing info")?;
            println!("{as_json}");
        }
    }

    Ok(())
}
