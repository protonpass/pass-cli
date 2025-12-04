use anyhow::{Context, Result, bail};
use clap::Args;
use pass::PassClient;
use pass::custom::{
    CustomFieldContentPayload, CustomFieldPayload, CustomItemCreatePayload, CustomSectionPayload,
};
use std::io::{self, Read};

use crate::commands::item::common::ShareQuery;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct CustomTemplate {
    pub title: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub sections: Vec<SectionTemplate>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SectionTemplate {
    pub section_name: String,
    #[serde(default)]
    pub fields: Vec<FieldTemplate>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct FieldTemplate {
    pub field_name: String,
    pub field_type: String,
    pub value: String,
}

impl Default for CustomTemplate {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            note: Some("".to_string()),
            sections: vec![SectionTemplate {
                section_name: "Section 1".to_string(),
                fields: vec![
                    FieldTemplate {
                        field_name: "Text Field".to_string(),
                        field_type: "text".to_string(),
                        value: "".to_string(),
                    },
                    FieldTemplate {
                        field_name: "Hidden Field".to_string(),
                        field_type: "hidden".to_string(),
                        value: "".to_string(),
                    },
                    FieldTemplate {
                        field_name: "TOTP Field".to_string(),
                        field_type: "totp".to_string(),
                        value: "otpauth://...".to_string(),
                    },
                    FieldTemplate {
                        field_name: "Timestamp Field".to_string(),
                        field_type: "timestamp".to_string(),
                        value: "1730000000".to_string(),
                    },
                ],
            }],
        }
    }
}

fn parse_field_type(field: FieldTemplate) -> Result<CustomFieldPayload> {
    let content = match field.field_type.to_lowercase().as_str() {
        "text" => CustomFieldContentPayload::Text(field.value),
        "hidden" => CustomFieldContentPayload::Hidden(field.value),
        "totp" => CustomFieldContentPayload::Totp(field.value),
        "timestamp" => {
            let timestamp = field
                .value
                .parse::<i64>()
                .with_context(|| format!("Invalid timestamp value '{}' for field '{}'. Must be a valid Unix timestamp (seconds since epoch)", field.value, field.field_name))?;
            CustomFieldContentPayload::Timestamp(timestamp)
        }
        _ => bail!(
            "Invalid field type '{}' for field '{}'. Valid types: text, hidden, totp, timestamp",
            field.field_type,
            field.field_name
        ),
    };

    Ok(CustomFieldPayload {
        field_name: field.field_name,
        content,
    })
}

impl CustomTemplate {
    fn into_payload(self) -> Result<CustomItemCreatePayload> {
        let sections: Result<Vec<CustomSectionPayload>> = self
            .sections
            .into_iter()
            .map(|section| {
                let section_fields: Result<Vec<CustomFieldPayload>> =
                    section.fields.into_iter().map(parse_field_type).collect();

                Ok(CustomSectionPayload {
                    section_name: section.section_name,
                    section_fields: section_fields?,
                })
            })
            .collect();

        Ok(CustomItemCreatePayload {
            title: self.title,
            note: self.note,
            sections: sections?,
        })
    }
}

#[derive(Args)]
pub struct CustomArgs {
    /// Display a template JSON structure for creating custom items
    #[arg(long, conflicts_with_all = ["from_template", "share_id"])]
    get_template: bool,

    /// Create from template file (use '-' for stdin)
    #[arg(long)]
    from_template: Option<String>,

    /// Share ID of the vault to create the custom item in
    #[arg(long)]
    share_id: Option<String>,

    /// Name of the vault to create the custom item in
    #[arg(long, help = "Name of the vault to create the custom item in")]
    vault_name: Option<String>,

    /// Folder ID to create the item in
    #[cfg(feature = "internal")]
    #[arg(long, help = "Folder ID to create the item in")]
    folder_id: Option<String>,
}

pub async fn run(args: CustomArgs, client: PassClient) -> Result<()> {
    if args.get_template {
        let template = CustomTemplate::default();
        let json = serde_json::to_string_pretty(&template)
            .context("Error serializing template to JSON")?;
        println!("{}", json);
        return Ok(());
    }

    let template_path = args
        .from_template
        .ok_or_else(|| anyhow::anyhow!("--from-template is required when not using --get-template. Use --get-template to see the JSON structure."))?;

    let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

    #[cfg(feature = "internal")]
    let folder_id = args
        .folder_id
        .as_ref()
        .map(|id| pass_domain::FolderId::new(id.clone()));
    #[cfg(not(feature = "internal"))]
    let folder_id = None;

    create_custom_from_template(&template_path, share_query, folder_id, client).await
}

async fn create_custom_from_template(
    template_path: &str,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let template_json = if template_path == "-" {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Error reading template from stdin")?;
        buffer
    } else {
        std::fs::read_to_string(template_path)
            .with_context(|| format!("Error reading template file: {}", template_path))?
    };

    let template: CustomTemplate = serde_json::from_str(&template_json)
        .context("Error parsing template JSON. Use --get-template to see the expected format")?;

    let payload = template
        .into_payload()
        .context("Error converting template to payload")?;

    create_custom_from_payload(payload, share_query, folder_id, client).await
}

async fn create_custom_from_payload(
    payload: CustomItemCreatePayload,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let item_id = client
        .create_custom(&share_id, payload, folder_id.as_ref())
        .await
        .context("Error creating custom item")?;

    println!("{}", item_id.value());
    Ok(())
}
