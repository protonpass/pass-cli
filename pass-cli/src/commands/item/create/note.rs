use anyhow::{Context, Result, bail};
use clap::Args;
use pass::PassClient;
use pass::note::NoteItemCreatePayload;
use std::io::{self, Read};

use crate::commands::item::common::ShareQuery;

#[derive(Debug, serde::Deserialize, serde::Serialize, Default)]
pub struct NoteTemplate {
    pub title: String,
    pub note: Option<String>,
}

impl From<NoteTemplate> for NoteItemCreatePayload {
    fn from(value: NoteTemplate) -> Self {
        Self {
            title: value.title,
            note: value.note,
        }
    }
}

#[derive(Args, Default, PartialEq, Eq)]
pub struct NoteArgs {
    /// Get a template JSON structure for creating note items
    #[arg(long, help = "Output a JSON template structure")]
    get_template: bool,

    /// Create note from template file or stdin
    #[arg(long, help = "Path to template file, or '-' for stdin")]
    from_template: Option<String>,

    /// Share ID of the vault to create the note item in
    #[arg(long, help = "Share ID of the vault to create the note item in")]
    share_id: Option<String>,

    /// Name of the vault to create the note item in
    #[arg(long, help = "Name of the vault to create the note item in")]
    vault_name: Option<String>,

    /// Title of the note item (required when not using template)
    #[arg(long, help = "Title of the note item")]
    title: Option<String>,

    /// Note content
    #[arg(long, help = "Note content")]
    note: Option<String>,

    /// Folder ID to create the item in
    #[cfg(feature = "internal")]
    #[arg(long, help = "Folder ID to create the item in")]
    folder_id: Option<String>,
}

pub async fn run(args: NoteArgs, client: PassClient) -> Result<()> {
    // Show help if no arguments provided
    if args.eq(&NoteArgs::default()) {
        bail!(
            "No arguments provided. Use 'pass-cli item create note --help' to see available options."
        );
    }

    // Handle get-template option
    if args.get_template {
        let template = NoteTemplate::default();
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
            serde_json::from_str::<NoteTemplate>(&contents)
                .context("Error parsing JSON from stdin")?
        } else {
            // Read from file
            let contents = std::fs::read_to_string(&template_source)
                .with_context(|| format!("Error reading template file: {template_source}"))?;
            serde_json::from_str::<NoteTemplate>(&contents)
                .with_context(|| format!("Error parsing JSON from file: {template_source}"))?
        };

        #[cfg(feature = "internal")]
        let folder_id = args
            .folder_id
            .as_ref()
            .map(|id| pass_domain::FolderId::new(id.clone()));
        #[cfg(not(feature = "internal"))]
        let folder_id = None;

        return create_note_from_template(template, share_query, folder_id, client).await;
    }

    // Handle individual field arguments
    let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

    let title = args
        .title
        .ok_or_else(|| anyhow::anyhow!("--title is required when not using --from-template"))?;

    let template = NoteTemplate {
        title,
        note: args.note,
    };

    #[cfg(feature = "internal")]
    let folder_id = args
        .folder_id
        .as_ref()
        .map(|id| pass_domain::FolderId::new(id.clone()));
    #[cfg(not(feature = "internal"))]
    let folder_id = None;

    create_note_from_template(template, share_query, folder_id, client).await
}

async fn create_note_from_template(
    template: NoteTemplate,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let res = client
        .create_note(&share_id, template.into(), folder_id.as_ref())
        .await
        .context("Error creating note item")?;
    println!("{res}");

    Ok(())
}
