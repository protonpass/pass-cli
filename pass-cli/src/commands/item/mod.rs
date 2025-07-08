use crate::commands::{OutputFormat, Role};
use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;
use pass_domain::{ItemId, ShareId};

pub mod attachment;
pub mod create;
pub mod delete;
pub mod list;
pub mod share;
pub mod view;

#[derive(Subcommand)]
pub enum ItemCommands {
    #[command(about = "List items in a vault")]
    List {
        #[arg(long, help = "Share ID of the vault to list items from")]
        share_id: String,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Create a new item")]
    Create {
        #[arg(long, help = "ID of the vault to create the item in")]
        vault: String,
    },
    #[command(about = "Delete an item")]
    Delete {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: String,
        #[arg(long, help = "ID of the item to delete")]
        item_id: String,
    },
    #[command(about = "Share an item")]
    Share {
        #[arg(long, help = "ID of the vault containing the item")]
        vault: String,
        #[arg(long, help = "ID of the item to share")]
        item: String,
        #[arg(long, default_value = "viewer")]
        role: Role,
    },
    #[command(about = "View an item")]
    View {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: String,
        #[arg(long, help = "ID of the item to view")]
        item_id: String,
        #[arg(long, help = "Specific field to view")]
        field: Option<String>,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Attachment operations")]
    Attachment {
        #[command(subcommand)]
        attachment_command: attachment::AttachmentCommands,
    },
}

pub async fn run(subcommand: ItemCommands, client: PassClient) -> Result<()> {
    match subcommand {
        ItemCommands::List { share_id, output } => {
            list::run(client, ShareId::new(share_id), output).await
        }
        ItemCommands::Create { vault } => create::run(client, vault).await,
        ItemCommands::Delete { share_id, item_id } => {
            delete::run(client, ShareId::new(share_id), ItemId::new(item_id)).await
        }
        ItemCommands::Share { vault, item, role } => share::run(client, vault, item, role).await,
        ItemCommands::View {
            share_id,
            item_id,
            field,
            output,
        } => {
            view::run(
                client,
                ShareId::new(share_id),
                ItemId::new(item_id),
                field,
                output,
            )
            .await
        }
        ItemCommands::Attachment { attachment_command } => {
            attachment::run(attachment_command, client).await
        }
    }
}
