use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;

pub mod set;
pub mod unset;
pub mod view;

/// Enum representing all available user settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    DefaultShareId,
    DefaultFormat,
}

impl Setting {
    /// Returns the setting key name used in the database
    pub fn key(&self) -> &'static str {
        match self {
            Setting::DefaultShareId => "default_share_id",
            Setting::DefaultFormat => "default_format",
        }
    }

    /// Returns the default value for this setting
    pub fn default_value(&self) -> &'static str {
        match self {
            Setting::DefaultShareId => "(none)",
            Setting::DefaultFormat => "human",
        }
    }

    /// Returns all available settings
    pub fn all() -> Vec<Setting> {
        vec![Setting::DefaultShareId, Setting::DefaultFormat]
    }
}

#[derive(Subcommand)]
pub enum SettingsCommands {
    #[command(about = "View all current settings")]
    View,

    #[command(about = "Set a setting value", subcommand)]
    Set(SetCommands),

    #[command(about = "Unset (clear) a setting", subcommand)]
    Unset(UnsetCommands),
}

#[derive(Subcommand)]
pub enum SetCommands {
    #[command(about = "Set the default vault")]
    DefaultVault {
        #[arg(long, help = "Vault name to set as default")]
        vault_name: Option<String>,
        #[arg(long, help = "Share ID to set as default")]
        share_id: Option<String>,
    },

    #[command(about = "Set the default output format")]
    DefaultFormat {
        #[arg(help = "Output format (human or json)")]
        format: String,
    },
}

#[derive(Subcommand)]
pub enum UnsetCommands {
    #[command(about = "Unset the default vault")]
    DefaultVault,

    #[command(about = "Unset the default output format")]
    DefaultFormat,
}

pub async fn run(subcommand: SettingsCommands, client: PassClient) -> Result<()> {
    match subcommand {
        SettingsCommands::View => view::run(client).await,
        SettingsCommands::Set(cmd) => set::run(cmd, client).await,
        SettingsCommands::Unset(cmd) => unset::run(cmd, client).await,
    }
}
