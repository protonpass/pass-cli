use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::{Item, ShareId};

#[derive(serde::Serialize)]
struct ItemsList {
    items: Vec<Item>,
}

pub async fn run(client: PassClient, share_id: ShareId, output: OutputFormat) -> Result<()> {
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
