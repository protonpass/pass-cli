use crate::commands::OutputFormat;
use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_domain::{Item, ShareId};

#[derive(serde::Serialize)]
struct ItemsList {
    items: Vec<Item>,
}

pub enum ListItemsQuery {
    ShareId(ShareId),
    VaultName(String),
}

impl ListItemsQuery {
    pub fn new(share_id: Option<String>, name: Option<String>) -> Result<Self> {
        match (share_id, name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),

            _ => Err(anyhow!("Please provide either share-id or vault name")),
        }
    }
}

pub async fn run(client: PassClient, query: ListItemsQuery, output: OutputFormat) -> Result<()> {
    let share_id = match query {
        ListItemsQuery::ShareId(id) => id,
        ListItemsQuery::VaultName(vault) => {
            let vault = client
                .find_vault(&vault)
                .await
                .context("Error finding vault")?;
            vault.share_id
        }
    };
    let items = client
        .list_items(&share_id)
        .await
        .context("Error listing items")?;

    match output {
        OutputFormat::Json => {
            let list = ItemsList { items };
            let json = serde_json::to_string_pretty(&list).context("Error serializing items")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            for item in items {
                println!(
                    "- [{}]: {} (state={:?})",
                    item.id, item.content.title, item.state
                );
            }
        }
    }

    Ok(())
}
