use crate::commands::OutputFormat;
use crate::commands::secret_resolver::ItemReference;
use anyhow::{Context, Result, anyhow, bail};
use pass::{FindItemQuery, PassClient};
use pass_domain::{ItemId, ShareId};

pub(crate) enum ShareQuery {
    ShareId(ShareId),
    VaultName(String),
}

pub(crate) enum ItemQuery {
    ItemId(ItemId),
    ItemTitle(String),
}

pub enum ViewItemQuery {
    Ids {
        share_query: ShareQuery,
        item_query: ItemQuery,
        field: Option<String>,
    },
    Uri(String),
}

impl ViewItemQuery {
    pub fn new(
        share_id: Option<String>,
        vault_name: Option<String>,
        item_id: Option<String>,
        item_title: Option<String>,
        field: Option<String>,
        uri: Option<String>,
    ) -> Result<Self> {
        // If URI is provided, that's the only valid combination
        if let Some(uri_value) = uri {
            if share_id.is_some()
                || vault_name.is_some()
                || item_id.is_some()
                || item_title.is_some()
            {
                return Err(anyhow!(
                    "When using URI, do not provide share-id, vault-name, item-id, or item-title"
                ));
            }
            return Ok(Self::Uri(uri_value));
        }

        // Otherwise, we need exactly one share identifier and one item identifier
        let share_query = match (share_id, vault_name) {
            (Some(share_id), None) => ShareQuery::ShareId(ShareId::new(share_id)),
            (None, Some(vault_name)) => ShareQuery::VaultName(vault_name),
            (None, None) => {
                return Err(anyhow!("Please provide either --share-id or --vault-name"));
            }
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "Please provide either --share-id or --vault-name, not both"
                ));
            }
        };

        let item_query = match (item_id, item_title) {
            (Some(item_id), None) => ItemQuery::ItemId(ItemId::new(item_id)),
            (None, Some(item_title)) => ItemQuery::ItemTitle(item_title),
            (None, None) => return Err(anyhow!("Please provide either --item-id or --item-title")),
            (Some(_), Some(_)) => {
                return Err(anyhow!(
                    "Please provide either --item-id or --item-title, not both"
                ));
            }
        };

        Ok(Self::Ids {
            share_query,
            item_query,
            field,
        })
    }
}

pub async fn run(client: PassClient, query: ViewItemQuery, output: OutputFormat) -> Result<()> {
    let (item, effective_field) = match query {
        ViewItemQuery::Ids {
            share_query,
            item_query,
            field,
        } => {
            // First, resolve the share_id
            let share_id = match share_query {
                ShareQuery::ShareId(id) => id,
                ShareQuery::VaultName(vault_name) => {
                    let vault = client
                        .find_vault(&vault_name)
                        .await
                        .context("Error finding vault")?;
                    vault.share_id
                }
            };

            // Then, resolve the item_id
            let item_id = match item_query {
                ItemQuery::ItemId(id) => id,
                ItemQuery::ItemTitle(title) => {
                    let items = client
                        .list_items(&share_id)
                        .await
                        .context("Error listing items")?;

                    let matching_item = items
                        .iter()
                        .find(|item| item.content.title == title)
                        .ok_or_else(|| anyhow!("No item found with title: {}", title))?;

                    matching_item.id.clone()
                }
            };

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
                if !item.item.content.note.is_empty() {
                    println!("- Note: {}", item.item.content.note);
                }
                println!("------");
                let content = item.item.content.pretty_print();
                if !content.is_empty() {
                    println!("{}", content);
                    println!("------");
                }
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
