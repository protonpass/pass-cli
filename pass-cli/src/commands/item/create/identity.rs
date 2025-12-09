use anyhow::{Context, Result};
use clap::Args;
use pass::PassClient;
use pass::identity::IdentityItemCreatePayload;
use std::io::{self, Read};

use crate::commands::item::common::ShareQuery;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct IdentityTemplate {
    pub title: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone_number: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub middle_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub birthdate: Option<String>,
    #[serde(default)]
    pub gender: Option<String>,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub street_address: Option<String>,
    #[serde(default)]
    pub zip_or_postal_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state_or_province: Option<String>,
    #[serde(default)]
    pub country_or_region: Option<String>,
    #[serde(default)]
    pub floor: Option<String>,
    #[serde(default)]
    pub county: Option<String>,
    #[serde(default)]
    pub social_security_number: Option<String>,
    #[serde(default)]
    pub passport_number: Option<String>,
    #[serde(default)]
    pub license_number: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub x_handle: Option<String>,
    #[serde(default)]
    pub second_phone_number: Option<String>,
    #[serde(default)]
    pub linkedin: Option<String>,
    #[serde(default)]
    pub reddit: Option<String>,
    #[serde(default)]
    pub facebook: Option<String>,
    #[serde(default)]
    pub yahoo: Option<String>,
    #[serde(default)]
    pub instagram: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub job_title: Option<String>,
    #[serde(default)]
    pub personal_website: Option<String>,
    #[serde(default)]
    pub work_phone_number: Option<String>,
    #[serde(default)]
    pub work_email: Option<String>,
}

impl Default for IdentityTemplate {
    fn default() -> Self {
        Self {
            title: "".to_string(),
            note: Some("".to_string()),
            full_name: Some("".to_string()),
            email: Some("".to_string()),
            phone_number: Some("".to_string()),
            first_name: Some("".to_string()),
            middle_name: Some("".to_string()),
            last_name: Some("".to_string()),
            birthdate: Some("".to_string()),
            gender: Some("".to_string()),
            organization: Some("".to_string()),
            street_address: Some("".to_string()),
            zip_or_postal_code: Some("".to_string()),
            city: Some("".to_string()),
            state_or_province: Some("".to_string()),
            country_or_region: Some("".to_string()),
            floor: Some("".to_string()),
            county: Some("".to_string()),
            social_security_number: Some("".to_string()),
            passport_number: Some("".to_string()),
            license_number: Some("".to_string()),
            website: Some("".to_string()),
            x_handle: Some("".to_string()),
            second_phone_number: Some("".to_string()),
            linkedin: Some("".to_string()),
            reddit: Some("".to_string()),
            facebook: Some("".to_string()),
            yahoo: Some("".to_string()),
            instagram: Some("".to_string()),
            company: Some("".to_string()),
            job_title: Some("".to_string()),
            personal_website: Some("".to_string()),
            work_phone_number: Some("".to_string()),
            work_email: Some("".to_string()),
        }
    }
}

impl From<IdentityTemplate> for IdentityItemCreatePayload {
    fn from(value: IdentityTemplate) -> Self {
        Self {
            title: value.title,
            note: value.note,
            full_name: value.full_name,
            email: value.email,
            phone_number: value.phone_number,
            first_name: value.first_name,
            middle_name: value.middle_name,
            last_name: value.last_name,
            birthdate: value.birthdate,
            gender: value.gender,
            organization: value.organization,
            street_address: value.street_address,
            zip_or_postal_code: value.zip_or_postal_code,
            city: value.city,
            state_or_province: value.state_or_province,
            country_or_region: value.country_or_region,
            floor: value.floor,
            county: value.county,
            social_security_number: value.social_security_number,
            passport_number: value.passport_number,
            license_number: value.license_number,
            website: value.website,
            x_handle: value.x_handle,
            second_phone_number: value.second_phone_number,
            linkedin: value.linkedin,
            reddit: value.reddit,
            facebook: value.facebook,
            yahoo: value.yahoo,
            instagram: value.instagram,
            company: value.company,
            job_title: value.job_title,
            personal_website: value.personal_website,
            work_phone_number: value.work_phone_number,
            work_email: value.work_email,
        }
    }
}

#[derive(Args)]
pub struct IdentityArgs {
    /// Display a template JSON structure for creating identity items
    #[arg(long, conflicts_with_all = ["from_template", "share_id"])]
    get_template: bool,

    /// Create from template file (use '-' for stdin)
    #[arg(long)]
    from_template: Option<String>,

    /// Share ID of the vault to create the identity item in
    #[arg(long)]
    share_id: Option<String>,

    /// Name of the vault to create the identity item in
    #[arg(long, help = "Name of the vault to create the identity item in")]
    vault_name: Option<String>,

    /// Folder ID to create the item in
    #[cfg(feature = "internal")]
    #[arg(long, help = "Folder ID to create the item in")]
    folder_id: Option<String>,
}

pub async fn run(args: IdentityArgs, client: PassClient) -> Result<()> {
    if args.get_template {
        let template = IdentityTemplate::default();
        let json = serde_json::to_string_pretty(&template)
            .context("Error serializing template to JSON")?;
        println!("{}", json);
        return Ok(());
    }

    let template_path = args.from_template.ok_or_else(|| {
        anyhow::anyhow!(
            "--from-template is required when not using --get-template. Use --get-template to see the JSON structure."
        )
    })?;

    let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

    #[cfg(feature = "internal")]
    let folder_id = args
        .folder_id
        .as_ref()
        .map(|id| pass_domain::FolderId::new(id.clone()));
    #[cfg(not(feature = "internal"))]
    let folder_id = None;

    create_identity_from_template(&template_path, share_query, folder_id, client).await
}

async fn create_identity_from_template(
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

    let template: IdentityTemplate = serde_json::from_str(&template_json)
        .context("Error parsing template JSON. Use --get-template to see the expected format")?;

    let payload = template.into();

    create_identity_from_payload(payload, share_query, folder_id, client).await
}

async fn create_identity_from_payload(
    payload: IdentityItemCreatePayload,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let item_id = client
        .create_identity(&share_id, payload, folder_id.as_ref())
        .await
        .context("Error creating identity item")?;

    println!("{}", item_id.value());
    Ok(())
}
