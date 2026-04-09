use super::super::VaultQuery;
use crate::commands::OutputFormat;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};

pub async fn run(client: PassClient, query: VaultQuery, output: OutputFormat) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    let members = client
        .list_vault_members(&share_id)
        .await
        .context("Error retrieving vault members")?;
    match output {
        OutputFormat::Human => {
            for member in members {
                println!("- {}: {:?}", member.email, member.role);
            }
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&members)?);
        }
    }
    Ok(())
}
