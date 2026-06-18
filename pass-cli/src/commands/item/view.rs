/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::commands::item::agent_monitor::send_reason_if_agent_with_name;
use crate::commands::item::common::{ItemQuery, ShareQuery};
use crate::commands::item::totp::generate_totp_token;
use crate::commands::secret_resolver::{ItemReference, TotpOutput};
use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use crate::telemetry::event::CommandEvent;
use anyhow::{Context, Result, anyhow, bail};
use pass::FindItemQuery;
use pass_domain::{EventAction, Field};

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

        let share_query = ShareQuery::new(share_id, vault_name)?;
        let item_query = ItemQuery::new(item_id, item_title)?;

        Ok(Self::Ids {
            share_query,
            item_query,
            field,
        })
    }
}

pub async fn run(
    client: PassClient,
    query: ViewItemQuery,
    output: Option<OutputFormat>,
) -> Result<()> {
    // Resolve output format from settings if not provided
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    let (item, effective_field, totp) = match query {
        ViewItemQuery::Ids {
            share_query,
            item_query,
            field,
        } => {
            client
                .emit_telemetry(&CommandEvent::new("item-view-args"))
                .await;
            let share_id = share_query.share_id(&client).await?;
            let item_id = item_query.item_id(&share_id, &client).await?;
            let item = client
                .view_item(&share_id, &item_id)
                .await
                .context("Error retrieving item")?;
            send_reason_if_agent_with_name(
                &client,
                EventAction::ItemRead,
                &share_id,
                Some(&item_id),
                Some(&item.item.content.title),
            )
            .await?;
            (item, field, None)
        }
        ViewItemQuery::Uri(uri) => {
            client
                .emit_telemetry(&CommandEvent::new("item-view-uri"))
                .await;
            let reference = ItemReference::parse(&uri).context("Invalid item reference")?;
            let totp = reference.totp;
            let item_query = FindItemQuery::new(&reference.share_id, &reference.item_id);
            let item = client
                .find_item(item_query)
                .await
                .context("Error retrieving item")?;
            let full_item = client
                .view_item(&item.share_id, &item.id)
                .await
                .context("Error fetching item details")?;
            send_reason_if_agent_with_name(
                &client,
                EventAction::ItemRead,
                &item.share_id,
                Some(&item.id),
                Some(&full_item.item.content.title),
            )
            .await?;
            (full_item, reference.field_name, totp)
        }
    };

    if let Some(field) = effective_field {
        match item.item.get_field(&field) {
            Some(Field::Totp(totp_uri)) => {
                let output = totp.unwrap_or_default();
                let value = match output {
                    TotpOutput::Uri => totp_uri,
                    TotpOutput::Code => generate_totp_token(&totp_uri)?,
                };
                println!("{}", value);
            }
            Some(field_value) => println!("{}", field_value.value()),
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
