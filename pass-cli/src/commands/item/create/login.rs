use anyhow::{Context, Result, bail};
use clap::Args;
use pass::login::LoginItemCreatePayload;
use pass::{
    PassClient,
    password::{PassphraseConfig, PasswordGenerationArgs, RandomPasswordConfig, WordSeparator},
};
use std::io::{self, Read};

use crate::commands::item::common::ShareQuery;

#[derive(Debug, serde::Deserialize, serde::Serialize, Default)]
pub struct LoginTemplate {
    pub title: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    #[serde(default)]
    pub urls: Vec<String>,
}

impl From<LoginTemplate> for LoginItemCreatePayload {
    fn from(value: LoginTemplate) -> Self {
        Self {
            title: value.title,
            email: value.email,
            username: value.username,
            password: value.password,
            urls: value.urls,
        }
    }
}

#[derive(Args, Default, PartialEq, Eq)]
pub struct LoginArgs {
    /// Get a template JSON structure for creating login items
    #[arg(long, help = "Output a JSON template structure")]
    get_template: bool,

    /// Create login from template file or stdin
    #[arg(long, help = "Path to template file, or '-' for stdin")]
    from_template: Option<String>,

    /// Share ID of the vault to create the login item in
    #[arg(long, help = "Share ID of the vault to create the login item in")]
    share_id: Option<String>,

    /// Name of the vault to create the login item in
    #[arg(long, help = "Name of the vault to create the login item in")]
    vault_name: Option<String>,

    /// Title of the login item (required when not using template)
    #[arg(long, help = "Title of the login item")]
    title: Option<String>,

    /// Username for the login
    #[arg(long, help = "Username for the login")]
    username: Option<String>,

    /// Email for the login
    #[arg(long, help = "Email for the login")]
    email: Option<String>,

    /// Password for the login
    #[arg(long, help = "Password for the login")]
    password: Option<String>,

    /// Generate a random password (optionally with custom settings: "length,uppercase,symbols")
    #[arg(long, help = "Generate a random password (optionally with custom settings: \"length,uppercase,symbols\")", action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "", require_equals = true, value_name = "SETTINGS")]
    generate_password: Option<String>,

    /// Generate a passphrase (optionally with custom word count)
    #[arg(long, help = "Generate a passphrase (optionally with custom word count)", action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "", require_equals = true, value_name = "WORD_COUNT")]
    generate_passphrase: Option<String>,

    /// URLs associated with the login
    #[arg(
        long,
        help = "URLs associated with the login (can be specified multiple times)"
    )]
    url: Vec<String>,

    /// Folder ID to create the item in
    #[cfg(feature = "internal")]
    #[arg(long, help = "Folder ID to create the item in")]
    folder_id: Option<String>,
}

pub async fn run(args: LoginArgs, client: PassClient) -> Result<()> {
    // Show help if no arguments provided
    if args.eq(&LoginArgs::default()) {
        bail!(
            "No arguments provided. Use 'pass-cli item create login --help' to see available options."
        );
    }

    // Handle get-template option
    if args.get_template {
        let template = LoginTemplate::default();
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
            serde_json::from_str::<LoginTemplate>(&contents)
                .context("Error parsing JSON from stdin")?
        } else {
            // Read from file
            let contents = std::fs::read_to_string(&template_source)
                .with_context(|| format!("Error reading template file: {template_source}"))?;
            serde_json::from_str::<LoginTemplate>(&contents)
                .with_context(|| format!("Error parsing JSON from file: {template_source}"))?
        };

        #[cfg(feature = "internal")]
        let folder_id = args
            .folder_id
            .as_ref()
            .map(|id| pass_domain::FolderId::new(id.clone()));
        #[cfg(not(feature = "internal"))]
        let folder_id = None;

        return create_login_from_template(template, share_query, folder_id, client).await;
    }

    // Handle individual field arguments
    let share_query = ShareQuery::new(args.share_id, args.vault_name)?;

    let title = args
        .title
        .ok_or_else(|| anyhow::anyhow!("--title is required when not using --from-template"))?;

    // Handle password generation options
    let password = if let Some(settings) = args.generate_password {
        if settings.is_empty() {
            Some(generate_default_password()?)
        } else {
            Some(generate_custom_password(&settings)?)
        }
    } else if let Some(word_count) = args.generate_passphrase {
        if word_count.is_empty() {
            Some(generate_default_passphrase()?)
        } else {
            Some(generate_custom_passphrase(&word_count)?)
        }
    } else {
        args.password
    };

    let template = LoginTemplate {
        title,
        username: args.username,
        email: args.email,
        password,
        urls: args.url,
    };

    #[cfg(feature = "internal")]
    let folder_id = args
        .folder_id
        .as_ref()
        .map(|id| pass_domain::FolderId::new(id.clone()));
    #[cfg(not(feature = "internal"))]
    let folder_id = None;

    create_login_from_template(template, share_query, folder_id, client).await
}

async fn create_login_from_template(
    template: LoginTemplate,
    share_query: ShareQuery,
    folder_id: Option<pass_domain::FolderId>,
    client: PassClient,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let res = client
        .create_login(&share_id, template.into(), folder_id.as_ref())
        .await
        .context("Error creating login item")?;
    println!("{res}");

    Ok(())
}

fn generate_default_password() -> Result<String> {
    let config = RandomPasswordConfig {
        length: 16,
        numbers: true,
        uppercase_letters: true,
        symbols: true,
    };

    pass::password::generate(PasswordGenerationArgs::Random(config))
        .context("Failed to generate default password")
}

fn generate_custom_password(custom_args: &str) -> Result<String> {
    let parts: Vec<&str> = custom_args.split(',').collect();

    if parts.is_empty() {
        bail!("Invalid password generation arguments: {}", custom_args);
    }

    let length = parts[0]
        .parse::<u32>()
        .with_context(|| format!("Invalid length: {}", parts[0]))?;

    let mut uppercase = false;
    let mut symbols = false;
    let numbers = true; // Default to true

    for part in parts.iter().skip(1) {
        match part.trim().to_lowercase().as_str() {
            "uppercase" => uppercase = true,
            "symbols" => symbols = true,
            _ => bail!("Unknown password option: {}", part),
        }
    }

    let config = RandomPasswordConfig {
        length,
        numbers,
        uppercase_letters: uppercase,
        symbols,
    };

    pass::password::generate(PasswordGenerationArgs::Random(config))
        .context("Failed to generate custom password")
}

fn generate_default_passphrase() -> Result<String> {
    let config = PassphraseConfig {
        separator: WordSeparator::Hyphens,
        capitalise: true,
        include_numbers: true,
        count: 5,
    };

    pass::password::generate(PasswordGenerationArgs::Passphrase(config))
        .context("Failed to generate default passphrase")
}

fn generate_custom_passphrase(word_count_str: &str) -> Result<String> {
    let count = word_count_str
        .parse::<u32>()
        .with_context(|| format!("Invalid word count: {word_count_str}"))?;

    let config = PassphraseConfig {
        separator: WordSeparator::Hyphens,
        capitalise: true,
        include_numbers: true,
        count,
    };

    pass::password::generate(PasswordGenerationArgs::Passphrase(config))
        .context("Failed to generate custom passphrase")
}
