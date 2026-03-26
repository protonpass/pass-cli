use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::Invite;

#[derive(serde::Serialize)]
struct InviteList {
    invites: Vec<InviteEntry>,
}

#[derive(serde::Serialize)]
struct InviteEntry {
    invite_type: String,
    inviter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl InviteEntry {
    pub fn from(invite: Invite) -> Self {
        let (invite_type, name) = match invite.vault_data {
            Some(data) => ("vault".to_string(), Some(data.vault_data.name)),
            None => ("item".to_string(), None),
        };

        Self {
            invite_type,
            inviter: invite.inviter_email,
            name,
        }
    }
}

pub async fn run(client: PassClient, output: OutputFormat) -> Result<()> {
    let invites = client
        .list_group_invites()
        .await
        .context("Error fetching group invites")?;
    let invites: Vec<Invite> = invites
        .into_iter()
        .map(|i| i.invite_with_keys.invite)
        .collect();

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
            let mapped: Vec<InviteEntry> = invites.into_iter().map(InviteEntry::from).collect();
            let instance = InviteList { invites: mapped };
            let as_json =
                serde_json::to_string_pretty(&instance).context("Error serializing invite list")?;

            println!("{}", as_json);
        }
    }

    Ok(())
}
