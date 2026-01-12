use crate::commands::OutputFormat;
use crate::commands::item::common::ShareQuery;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::ItemId;

#[derive(serde::Serialize)]
struct JsonAliasItem {
    id: ItemId,
    alias: String,
}

pub async fn run(
    client: PassClient,
    share_query: ShareQuery,
    prefix: String,
    output: OutputFormat,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let res = client
        .create_alias(&share_id, &prefix)
        .await
        .context("Error creating alias")?;

    match output {
        OutputFormat::Human => {
            println!("{}", res.alias);
        }
        OutputFormat::Json => {
            let res = JsonAliasItem {
                id: res.item_id,
                alias: res.alias,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&res).context("Error serializing output")?
            );
        }
    }

    Ok(())
}
