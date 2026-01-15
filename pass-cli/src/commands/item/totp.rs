use crate::commands::item::common::{ItemQuery, ShareQuery};
use crate::commands::secret_resolver::ItemReference;
use crate::commands::{OutputFormat, settings_helper};
use anyhow::{Context, Result, anyhow, bail};
use jiff::Timestamp;
use pass::{FindItemQuery, PassClient};
use pass_domain::Field;
use proton_pass_common::totp::TOTP;
use serde::Serialize;
use std::collections::HashMap;

pub enum ViewTotpQuery {
    Ids {
        share_query: ShareQuery,
        item_query: ItemQuery,
        field: Option<String>,
    },
    Uri(String),
}

impl ViewTotpQuery {
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
            (Some(share_id), None) => ShareQuery::ShareId(pass_domain::ShareId::new(share_id)),
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
            (Some(item_id), None) => ItemQuery::ItemId(pass_domain::ItemId::new(item_id)),
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

#[derive(Serialize)]
struct TotpOutput {
    #[serde(flatten)]
    tokens: HashMap<String, String>,
}

fn generate_totp_token(totp_uri: &str) -> Result<String> {
    let totp = TOTP::from_uri(totp_uri)
        .context("Failed to parse TOTP content. Please ensure the field contains a valid TOTP URI or base32 secret")?;

    let timestamp = Timestamp::now().as_second() as u64;
    let token = totp
        .generate_token(timestamp)
        .context("Failed to generate TOTP token")?;

    Ok(token)
}

pub async fn run(
    client: PassClient,
    query: ViewTotpQuery,
    output: Option<OutputFormat>,
) -> Result<()> {
    // Resolve output format from settings if not provided
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let (item, effective_field) = match query {
        ViewTotpQuery::Ids {
            share_query,
            item_query,
            field,
        } => {
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

            let item = match item_query {
                ItemQuery::ItemId(id) => {
                    let item = client
                        .view_item(&share_id, &id)
                        .await
                        .context("Error retrieving item")?;

                    item.item
                }
                ItemQuery::ItemTitle(title) => {
                    let items = client
                        .list_items(&share_id)
                        .await
                        .context("Error listing items")?;

                    items
                        .into_iter()
                        .find(|item| item.content.title == title)
                        .ok_or_else(|| anyhow!("No item found with title: {}", title))?
                }
            };
            (item, field)
        }
        ViewTotpQuery::Uri(uri) => {
            let reference = ItemReference::parse(&uri).context("Invalid item reference")?;
            let item_query = FindItemQuery::new(&reference.share_id, &reference.item_id);
            let item = client
                .find_item(item_query)
                .await
                .context("Error retrieving item")?;

            (item, reference.field_name)
        }
    };

    let mut totp_fields: HashMap<String, String> = HashMap::new();

    if let Some(field_name) = effective_field {
        // User specified a specific field
        match item.get_field(&field_name) {
            Some(Field::Totp(totp_uri)) => {
                let token = generate_totp_token(&totp_uri)?;
                totp_fields.insert(field_name, token);
            }
            Some(_) => {
                bail!("Field '{}' is not a TOTP field", field_name);
            }
            None => {
                bail!("Field does not exist: {}", field_name);
            }
        }
    } else {
        // No specific field, collect all TOTP fields
        for (field_name, field) in item.fields() {
            if let Field::Totp(totp_uri) = field
                && !totp_uri.is_empty()
            {
                match generate_totp_token(&totp_uri) {
                    Ok(token) => {
                        totp_fields.insert(field_name, token);
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to generate TOTP for field '{}': {}",
                            field_name, e
                        );
                    }
                }
            }
        }

        if totp_fields.is_empty() {
            bail!("No TOTP fields found in this item");
        }
    }

    // Output the results
    match output {
        OutputFormat::Human => {
            for (field_name, token) in &totp_fields {
                println!("{}: {}", field_name, token);
            }
        }
        OutputFormat::Json => {
            let output = TotpOutput {
                tokens: totp_fields,
            };
            let as_json =
                serde_json::to_string_pretty(&output).context("Error serializing TOTP output")?;
            println!("{}", as_json);
        }
    }

    Ok(())
}
