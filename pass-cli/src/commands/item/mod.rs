use crate::commands::item::list::{FilterState, FilterType, ListItemsQuery, SortBy};
use crate::commands::{OutputFormat, Role};
use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;
use pass_domain::{ItemId, ShareId};

pub mod alias;
pub mod attachment;
pub mod create;
pub mod delete;
pub mod list;
pub mod member;
pub mod share;
pub mod view;

#[derive(Subcommand)]
pub enum ItemCommands {
    #[command(about = "List items in a vault")]
    List {
        #[arg(long, help = "Share ID of the vault to list items from")]
        share_id: Option<String>,
        #[arg(help = "Name of the vault to list items from")]
        vault_name: Option<String>,
        #[arg(
            long,
            help = "Filter items by type (note, login, alias, credit-card, identity, ssh-key, wifi, custom)"
        )]
        filter_type: Option<FilterType>,
        #[arg(long, help = "Filter items by state (active, trashed)")]
        filter_state: Option<FilterState>,
        #[arg(
            long,
            help = "Sort items (alphabetic-asc, alphabetic-desc, created-asc, created-desc)"
        )]
        sort_by: Option<SortBy>,
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Create a new item")]
    Create {
        #[command(subcommand)]
        create_command: create::CreateCommands,
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
        #[arg(long, help = "Share ID that contains the item")]
        share_id: String,
        #[arg(long, help = "ID of the item to share")]
        item_id: String,
        #[arg(help = "Email address to share with")]
        email: String,
        #[arg(long, default_value = "viewer")]
        role: Role,
    },
    #[command(about = "View an item")]
    View {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault containing the item")]
        vault_name: Option<String>,
        #[arg(long, help = "ID of the item to view")]
        item_id: Option<String>,
        #[arg(long, help = "Title of the item to view")]
        item_title: Option<String>,
        #[arg(help = "Pass URI in format pass://SHARE_ID/ITEM_ID[/FIELD]")]
        uri: Option<String>,
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
    #[command(about = "Alias operations")]
    Alias {
        #[command(subcommand)]
        alias_command: alias::AliasCommands,
    },
    #[command(about = "Manage item members", subcommand)]
    Member(member::MemberCommands),
}

pub async fn run(subcommand: ItemCommands, client: PassClient) -> Result<()> {
    match subcommand {
        ItemCommands::List {
            share_id,
            vault_name,
            filter_type,
            filter_state,
            sort_by,
            output,
        } => {
            let query = ListItemsQuery::new(share_id, vault_name)?;
            list::run(client, query, filter_type, filter_state, sort_by, output).await
        }
        ItemCommands::Create { create_command } => create::run(create_command, client).await,
        ItemCommands::Delete { share_id, item_id } => {
            delete::run(client, ShareId::new(share_id), ItemId::new(item_id)).await
        }
        ItemCommands::Share {
            share_id,
            item_id,
            email,
            role,
        } => {
            share::run(
                client,
                ShareId::new(share_id),
                ItemId::new(item_id),
                &email,
                role,
            )
            .await
        }
        ItemCommands::View {
            share_id,
            vault_name,
            item_id,
            item_title,
            uri,
            field,
            output,
        } => {
            let query =
                view::ViewItemQuery::new(share_id, vault_name, item_id, item_title, field, uri)?;
            view::run(client, query, output).await
        }
        ItemCommands::Attachment { attachment_command } => {
            attachment::run(attachment_command, client).await
        }
        ItemCommands::Alias { alias_command } => alias::run(alias_command, client).await,
        ItemCommands::Member(member_cmd) => member::run(client, member_cmd).await,
    }
}
