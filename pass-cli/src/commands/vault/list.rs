use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::Vault;

#[derive(serde::Serialize)]
struct VaultList {
    pub vaults: Vec<VaultEntry>,
}

#[derive(serde::Serialize)]
struct VaultEntry {
    pub name: String,
    pub vault_id: String,
    pub share_id: String,
}

impl From<Vault> for VaultEntry {
    fn from(vault: Vault) -> Self {
        Self {
            name: vault.content.name,
            share_id: vault.share_id.to_string(),
            vault_id: vault.id.to_string(),
        }
    }
}

pub async fn run(client: PassClient, output: Option<OutputFormat>) -> Result<()> {
    // Resolve output format from settings if not provided
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let vaults = client.list_vaults().await.context("Error listing vaults")?;
    let list = VaultList {
        vaults: vaults.into_iter().map(VaultEntry::from).collect(),
    };
    print(list, output).context("Error printing vaults")?;

    Ok(())
}

fn print(vaults: VaultList, output: OutputFormat) -> Result<()> {
    match output {
        OutputFormat::Human => {
            for vault in vaults.vaults {
                println!("- [{}]: {}", vault.share_id, vault.name);
            }
        }
        OutputFormat::Json => {
            let as_json =
                serde_json::to_string_pretty(&vaults).context("Error serializing vaults")?;
            println!("{as_json}");
        }
    }

    Ok(())
}
