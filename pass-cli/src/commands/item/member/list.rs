use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::{ItemId, ShareId};

pub async fn run(
    client: PassClient,
    share_id: ShareId,
    item_id: ItemId,
    output: OutputFormat,
) -> Result<()> {
    let members = client
        .list_item_members(&share_id, &item_id)
        .await
        .context("Error retrieving item members")?;
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
