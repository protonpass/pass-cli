use crate::commands::item::common::ShareQuery;
use anyhow::{Context, Result, bail};
use clap::Args;
use pass::PassClient;
use pass::credit_card::CreditCardItemCreatePayload;
use std::io::{self, Read};

#[derive(Debug, serde::Deserialize, serde::Serialize, Default)]
pub struct CreditCardTemplate {
    pub title: String,
    pub cardholder_name: Option<String>,
    pub card_type: Option<String>,
    pub number: Option<String>,
    pub cvv: Option<String>,
    pub expiration_date: Option<String>,
    pub pin: Option<String>,
    pub note: Option<String>,
}

impl CreditCardTemplate {
    fn into_payload(self) -> CreditCardItemCreatePayload {
        CreditCardItemCreatePayload {
            title: self.title,
            cardholder_name: self.cardholder_name,
            number: self.number,
            verification_number: self.cvv,
            expiration_date: self.expiration_date,
            pin: self.pin,
            note: self.note,
        }
    }
}

#[derive(Args, Default, PartialEq, Eq)]
pub struct CreditCardArgs {
    /// Get a template JSON structure for creating credit card items
    #[arg(long, help = "Output a JSON template structure")]
    get_template: bool,

    /// Create credit card from template file or stdin
    #[arg(long, help = "Path to template file, or '-' for stdin")]
    from_template: Option<String>,

    /// Share ID of the vault to create the credit card item in
    #[arg(long, help = "Share ID of the vault to create the credit card item in")]
    share_id: Option<String>,

    /// Name of the vault to create the credit card item in
    #[arg(long, help = "Name of the vault to create the credit card item in")]
    vault_name: Option<String>,

    /// Title of the credit card item (required when not using template)
    #[arg(long, help = "Title of the credit card item")]
    title: Option<String>,

    /// Cardholder name
    #[arg(long, help = "Cardholder name")]
    cardholder_name: Option<String>,

    /// Card number
    #[arg(long, help = "Card number")]
    number: Option<String>,

    /// CVV/CVC security code
    #[arg(long, help = "CVV/CVC security code")]
    cvv: Option<String>,

    /// Expiration date in format YYYY-MM (e.g., 2027-12)
    #[arg(long, help = "Expiration date in format YYYY-MM (e.g., 2027-12)")]
    expiration_date: Option<String>,

    /// Card PIN
    #[arg(long, help = "Card PIN")]
    pin: Option<String>,

    /// Note content
    #[arg(long, help = "Note content")]
    note: Option<String>,

    /// Folder ID to create the item in
    #[cfg(feature = "internal")]
    #[arg(long, help = "Folder ID to create the item in")]
    folder_id: Option<String>,
}

pub async fn run(args: CreditCardArgs, client: PassClient) -> Result<()> {
    // Show help if no arguments provided
    if args.eq(&CreditCardArgs::default()) {
        bail!(
            "No arguments provided. Use 'pass-cli item create credit-card --help' to see available options."
        );
    }

    // Handle get-template option
    if args.get_template {
        let template = CreditCardTemplate::default();
        let json = serde_json::to_string_pretty(&template).context("Error serializing template")?;
        println!("{json}");
        return Ok(());
    }

    // Handle from-template option
    if let Some(template_source) = args.from_template {
        let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

        let template = if template_source == "-" {
            // Read from stdin
            let mut stdin = io::stdin();
            let mut contents = String::new();
            stdin
                .read_to_string(&mut contents)
                .context("Error reading from stdin")?;
            serde_json::from_str::<CreditCardTemplate>(&contents)
                .context("Error parsing JSON from stdin")?
        } else {
            // Read from file
            let contents = std::fs::read_to_string(&template_source)
                .with_context(|| format!("Error reading template file: {template_source}"))?;
            serde_json::from_str::<CreditCardTemplate>(&contents)
                .with_context(|| format!("Error parsing JSON from file: {template_source}"))?
        };

        #[cfg(feature = "internal")]
        let folder_id = args
            .folder_id
            .as_ref()
            .map(|id| pass_domain::FolderId::new(id.clone()));
        #[cfg(not(feature = "internal"))]
        let folder_id = None;

        return create_credit_card_from_template(template, share_query, folder_id, client).await;
    }

    // Handle individual field arguments
    let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

    let title = args
        .title
        .ok_or_else(|| anyhow::anyhow!("--title is required when not using --from-template"))?;

    let payload = CreditCardItemCreatePayload {
        title,
        cardholder_name: args.cardholder_name,
        number: args.number,
        verification_number: args.cvv,
        expiration_date: args.expiration_date,
        pin: args.pin,
        note: args.note,
    };

    #[cfg(feature = "internal")]
    let folder_id = args
        .folder_id
        .as_ref()
        .map(|id| pass_domain::FolderId::new(id.clone()));
    #[cfg(not(feature = "internal"))]
    let folder_id = None;

    create_credit_card_from_payload(payload, share_query, folder_id, client).await
}

async fn create_credit_card_from_template(
    template: CreditCardTemplate,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let payload = template.into_payload();
    create_credit_card_from_payload(payload, share_query, folder_id, client).await
}

async fn create_credit_card_from_payload(
    payload: CreditCardItemCreatePayload,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let res = client
        .create_credit_card(&share_id, payload, folder_id.as_ref())
        .await
        .context("Error creating credit card item")?;
    println!("{res}");

    Ok(())
}
