use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use pass::PassClient;

pub async fn run(client: PassClient, output: OutputFormat) -> Result<()> {
    let invites = client
        .list_user_invites()
        .await
        .context("Error listing invites")?;
    match output {
        OutputFormat::Human => {
            for invite in invites {
                match invite.vault_data {
                    Some(vault_data) => {
                        println!(
                            "- [{}]: Type=Vault | Vault={} | From {}",
                            invite.id, vault_data.vault_data.name, invite.inviter_email
                        );
                    }
                    // Item
                    None => {
                        println!(
                            "- [{}]: Type=Item | From {}",
                            invite.id, invite.inviter_email
                        );
                    }
                }
            }
        }
        OutputFormat::Json => {
            todo!()
        }
    }

    Ok(())
}
