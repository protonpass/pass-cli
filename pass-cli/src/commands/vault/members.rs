use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::ShareId;

pub async fn run(client: PassClient, share_id: ShareId, output: OutputFormat) -> Result<()> {
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
