use crate::commands::OutputFormat;
use crate::commands::secret_resolver::ItemReference;
use anyhow::{Context, Result, anyhow, bail};
use pass::{FindItemQuery, PassClient};
use pass_domain::{ItemId, ShareId};

pub enum ViewItemQuery {
    Ids {
        share_id: ShareId,
        item_id: ItemId,
        field: Option<String>,
    },
    Uri(String),
}

impl ViewItemQuery {
    pub fn new(
        share_id: Option<String>,
        item_id: Option<String>,
        field: Option<String>,
        uri: Option<String>,
    ) -> Result<Self> {
        match (share_id, item_id, uri) {
            (Some(share_id), Some(item_id), None) => Ok(Self::Ids {
                share_id: ShareId::new(share_id),
                item_id: ItemId::new(item_id),
                field,
            }),
            (None, None, Some(uri)) => Ok(Self::Uri(uri)),
            _ => Err(anyhow!(
                "Please provide either (share_id + item_id) or a uri"
            )),
        }
    }
}

pub async fn run(client: PassClient, query: ViewItemQuery, output: OutputFormat) -> Result<()> {
    let (item, effective_field) = match query {
        ViewItemQuery::Ids {
            share_id,
            item_id,
            field,
        } => {
            let item = client
                .view_item(&share_id, &item_id)
                .await
                .context("Error retrieving item")?;
            (item, field)
        }
        ViewItemQuery::Uri(uri) => {
            let reference = ItemReference::parse(&uri).context("Invalid item reference")?;
            let item_query = FindItemQuery::new(&reference.share_id, &reference.item_id);
            let item = client
                .find_item(item_query)
                .await
                .context("Error retrieving item")?;

            let full_item = client
                .view_item(&item.share_id, &item.id)
                .await
                .context("Error fetching item details")?;
            (full_item, reference.field_name)
        }
    };

    if let Some(field) = effective_field {
        match item.item.get_field(&field) {
            Some(field) => println!("{field}"),
            None => bail!("Field does not exist: {}", &field),
        }
    } else {
        match output {
            OutputFormat::Json => {
                let as_json =
                    serde_json::to_string_pretty(&item).context("Error serializing item")?;
                println!("{as_json}");
            }
            OutputFormat::Human => {
                println!("- Title: {}", item.item.content.title);
                println!("- ID: {}", item.item.id);
                println!("- ShareID: {}", item.item.share_id);
                println!("- Item ID: {}", item.item.id);
                println!("------");
                println!("{:#?}", item.item.content);
                println!("------");
                if !item.attachments.is_empty() {
                    println!("- Attachments:");
                    for attachment in item.attachments {
                        println!("--- Attachment name: {}", attachment.content.name);
                        println!(
                            "--- Attachment size: {}",
                            human_readable_size(attachment.size)
                        );
                        println!("--- Attachment type: {}", attachment.content.mime_type);
                        println!("--- Attachment ID: {}", attachment.id);
                        println!();
                    }
                }
            }
        };
    }

    Ok(())
}

fn human_readable_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as usize, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}
