use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::Invite;

pub async fn run(client: PassClient, output: OutputFormat) -> Result<()> {
    let invites = client
        .list_user_invites()
        .await
        .context("Error listing invites")?;

    let invites: Vec<Invite> = invites.into_iter().map(|i| i.invite).collect();
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
