use crate::commands::{OutputFormat, Role};
use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;
use pass_domain::ShareId;

pub mod create;
pub mod delete;
pub mod list;
pub mod member;
pub mod share;
mod update;

#[derive(Subcommand)]
pub enum VaultCommands {
    #[command(about = "List vaults")]
    List {
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Create a new vault")]
    Create {
        #[arg(long, help = "Name of the vault")]
        name: String,
    },
    #[command(about = "Update a vault")]
    Update {
        #[arg(long, help = "Share ID of the vault")]
        share_id: String,
        #[arg(long, help = "New name of the vault")]
        name: String,
    },
    #[command(about = "Manage vault members", subcommand)]
    Member(member::MemberCommands),
    #[command(about = "Delete a vault")]
    Delete {
        #[arg(long, help = "Share ID of the vault to delete")]
        share_id: String,
    },
    #[command(about = "Share a vault with someone")]
    Share {
        #[arg(long, help = "Share ID of the vault to share")]
        share_id: String,
        #[arg(help = "Email address to share with")]
        email: String,
        #[arg(long, default_value = "viewer")]
        role: Role,
    },
}

pub async fn run(subcommand: VaultCommands, client: PassClient) -> Result<()> {
    match subcommand {
        VaultCommands::List { output } => list::run(client, output).await,
        VaultCommands::Update { share_id, name } => {
            update::run(client, ShareId::new(share_id), name).await
        }
        VaultCommands::Create { name } => create::run(client, name).await,
        VaultCommands::Member(member_cmd) => member::run(client, member_cmd).await,
        VaultCommands::Delete { share_id } => delete::run(client, ShareId::new(share_id)).await,
        VaultCommands::Share {
            share_id,
            email,
            role,
        } => share::run(client, ShareId::new(share_id), email, role).await,
    }
}
