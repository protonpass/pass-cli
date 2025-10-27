use anyhow::{Context, Result, bail};
use clap::Args;
use pass::PassClient;
use pass::note::NoteItemCreatePayload;
use pass_domain::ShareId;
use std::io::{self, Read};

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

    /// Title of the note item (required when not using template)
    #[arg(long, help = "Title of the note item")]
    title: Option<String>,

    /// Note content
    #[arg(long, help = "Note content")]
    note: Option<String>,
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
        let share_id = args
            .share_id
            .ok_or_else(|| anyhow::anyhow!("--share-id is required when using --from-template"))?;

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

        return create_note_from_template(template, share_id, client).await;
    }

    // Handle individual field arguments
    let share_id = args
        .share_id
        .ok_or_else(|| anyhow::anyhow!("--share-id is required"))?;

    let title = args
        .title
        .ok_or_else(|| anyhow::anyhow!("--title is required when not using --from-template"))?;

    let template = NoteTemplate {
        title,
        note: args.note,
    };

    create_note_from_template(template, share_id, client).await
}

async fn create_note_from_template(
    template: NoteTemplate,
    share_id: String,
    client: PassClient,
) -> Result<()> {
    let share_id = ShareId::new(share_id);
    let res = client
        .create_note(&share_id, template.into())
        .await
        .context("Error creating note item")?;
    println!("{res}");

    Ok(())
}
