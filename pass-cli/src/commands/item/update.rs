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

use super::common::{ItemQuery, ShareQuery};
use crate::commands::item::agent_monitor::send_reason_if_agent;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::{EventAction, UpdateFieldResult};

fn parse_fields(fields: Vec<String>) -> Result<Vec<(String, String)>> {
    fields
        .iter()
        .map(|field_str| {
            let parts: Vec<&str> = field_str.splitn(2, '=').collect();
            if parts.len() != 2 {
                Err(anyhow!(
                    "Invalid field format '{}'. Expected 'field_name=field_value'",
                    field_str
                ))
            } else {
                Ok((parts[0].to_string(), parts[1].to_string()))
            }
        })
        .collect()
}

pub async fn run(
    client: PassClient,
    share_query: ShareQuery,
    item_query: ItemQuery,
    fields: Vec<String>,
) -> Result<()> {
    let parsed_fields = parse_fields(fields)?;

    if parsed_fields.is_empty() {
        return Err(anyhow!(
            "No fields to update. Use --field to specify at least one field to update"
        ));
    }

    let share_id = share_query.share_id(&client).await?;
    let item_id = item_query.item_id(&share_id, &client).await?;

    send_reason_if_agent(&client, EventAction::ItemUpdate, &share_id, Some(&item_id)).await?;

    let item_details = client
        .view_item(&share_id, &item_id)
        .await
        .context("Error retrieving item")?;

    let mut updated_content = item_details.item.content;

    // Track the results
    let mut fields_updated = 0;
    let mut fields_created = 0;

    // Update each field
    for (field_name, field_value) in parsed_fields {
        match updated_content.update_field(&field_name, &field_value) {
            Ok(UpdateFieldResult::FieldUpdated) => {
                println!("Updated field: {}", field_name);
                fields_updated += 1;
            }
            Ok(UpdateFieldResult::CustomFieldCreated) => {
                println!("Created new custom field: {}", field_name);
                fields_created += 1;
            }
            Err(e) => {
                return Err(anyhow!("Error updating field '{}': {}", field_name, e));
            }
        }
    }

    client
        .update_item(&share_id, &item_id, updated_content)
        .await
        .context("Error updating item")?;

    eprintln!(
        "Item updated successfully: {} field(s) updated, {} custom field(s) created",
        fields_updated, fields_created
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fields_empty() {
        let fields = parse_fields(vec![]).expect("should be able to parse fields");
        assert_eq!(fields.len(), 0);
    }

    #[test]
    fn parse_fields_single() {
        let fields = parse_fields(vec!["a=1".to_string()]).expect("should be able to parse fields");
        assert_eq!(fields.len(), 1);

        let field = fields.first().unwrap();
        assert_eq!(field.0, "a");
        assert_eq!(field.1, "1");
    }

    #[test]
    fn parse_fields_multiple() {
        let fields = parse_fields(vec!["a=1".to_string(), "b=2".to_string()])
            .expect("should be able to parse fields");
        assert_eq!(fields.len(), 2);

        let mut fields_iter = fields.into_iter();
        let first_field = fields_iter.next().unwrap();
        assert_eq!(first_field.0, "a");
        assert_eq!(first_field.1, "1");

        let second_field = fields_iter.next().unwrap();
        assert_eq!(second_field.0, "b");
        assert_eq!(second_field.1, "2");
    }

    #[test]
    fn parse_fields_no_equal() {
        let err =
            parse_fields(vec!["abc".to_string()]).expect_err("should not be able to parse fields");
        assert!(err.to_string().contains("Invalid field format"));
    }

    #[test]
    fn parse_fields_many_equals() {
        let fields =
            parse_fields(vec!["a=1=2".to_string()]).expect("should be able to parse fields");
        let field = fields.first().unwrap();
        assert_eq!(field.0, "a");
        assert_eq!(field.1, "1=2");
    }

    #[test]
    fn parse_fields_one_valid_one_error() {
        let err = parse_fields(vec!["a=1".to_string(), "b".to_string()])
            .expect_err("should not be able to parse fields");
        assert!(err.to_string().contains("Invalid field format"));
    }
}
