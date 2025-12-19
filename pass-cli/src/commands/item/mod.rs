use crate::commands::item::list::{FilterState, FilterType, ListItemsQuery, SortBy};
use crate::commands::{OutputFormat, Role, settings_helper};
use anyhow::{Result, anyhow};
use clap::Subcommand;
use pass::PassClient;
use pass_domain::{ItemId, ShareId};

pub mod alias;
pub mod attachment;
mod common;
pub mod create;
pub mod delete;
pub mod list;
pub mod member;
pub mod r#move;
pub mod share;
pub mod totp;
pub mod trash;
pub mod untrash;
pub mod update;
pub mod view;

// Re-export common types for use by other modules (used by internal folder commands)
#[allow(unused_imports)]
pub use common::{ItemQuery, ShareQuery};

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
        #[arg(long)]
        output: Option<OutputFormat>,
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
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    #[command(about = "Move an item to a different vault")]
    Move {
        #[arg(long, help = "Share ID of the source vault")]
        from_share_id: Option<String>,
        #[arg(long, help = "Name of the source vault")]
        from_vault_name: Option<String>,
        #[arg(long, help = "ID of the item to move")]
        item_id: Option<String>,
        #[arg(long, help = "Title of the item to move")]
        item_title: Option<String>,
        #[arg(long, help = "Share ID of the destination vault")]
        to_share_id: Option<String>,
        #[arg(long, help = "Name of the destination vault")]
        to_vault_name: Option<String>,
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
    #[command(about = "Generate TOTP code(s) for an item")]
    Totp {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault containing the item")]
        vault_name: Option<String>,
        #[arg(long, help = "ID of the item")]
        item_id: Option<String>,
        #[arg(long, help = "Title of the item")]
        item_title: Option<String>,
        #[arg(help = "Pass URI in format pass://SHARE_ID/ITEM_ID[/FIELD]")]
        uri: Option<String>,
        #[arg(long, help = "Specific TOTP field to generate code for")]
        field: Option<String>,
        #[arg(long)]
        output: Option<OutputFormat>,
    },
    #[command(about = "Move an item to trash")]
    Trash {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault containing the item")]
        vault_name: Option<String>,
        #[arg(long, help = "ID of the item to trash")]
        item_id: Option<String>,
        #[arg(long, help = "Title of the item to trash")]
        item_title: Option<String>,
    },
    #[command(about = "Restore an item from trash")]
    Untrash {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault containing the item")]
        vault_name: Option<String>,
        #[arg(long, help = "ID of the item to restore")]
        item_id: Option<String>,
        #[arg(long, help = "Title of the item to restore")]
        item_title: Option<String>,
    },
    #[command(about = "Update an item's fields")]
    Update {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: Option<String>,
        #[arg(long, help = "Name of the vault containing the item")]
        vault_name: Option<String>,
        #[arg(long, help = "ID of the item to update")]
        item_id: Option<String>,
        #[arg(long, help = "Title of the item to update")]
        item_title: Option<String>,
        #[arg(
            long = "field",
            help = "Field to update in format field_name=field_value (can be specified multiple times)"
        )]
        fields: Vec<String>,
    },
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
            let query = match (&share_id, &vault_name) {
                (None, None) => {
                    // Try to use default vault from settings
                    if let Some(default_share_id) =
                        settings_helper::get_default_share_id(&client).await?
                    {
                        ListItemsQuery::ShareId(default_share_id)
                    } else {
                        return Err(anyhow!(
                            "Please provide either --share-id, --vault-name, or set a default vault with 'pass-cli settings set default-vault'"
                        ));
                    }
                }
                _ => ListItemsQuery::new(share_id, vault_name)?,
            };
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
            // Apply default vault if both are None and URI is not provided
            let (share_id, vault_name) =
                if share_id.is_none() && vault_name.is_none() && uri.is_none() {
                    if let Some(default_share_id) =
                        settings_helper::get_default_share_id(&client).await?
                    {
                        (Some(default_share_id.to_string()), None)
                    } else {
                        (None, None)
                    }
                } else {
                    (share_id, vault_name)
                };

            let query =
                view::ViewItemQuery::new(share_id, vault_name, item_id, item_title, field, uri)?;
            view::run(client, query, output).await
        }
        ItemCommands::Move {
            from_share_id,
            from_vault_name,
            item_id,
            item_title,
            to_share_id,
            to_vault_name,
        } => {
            // Apply default vault to "from" vault if both are None
            let (from_share_id, from_vault_name) = if from_share_id.is_none()
                && from_vault_name.is_none()
            {
                if let Some(default_share_id) =
                    settings_helper::get_default_share_id(&client).await?
                {
                    (Some(default_share_id.to_string()), None)
                } else {
                    return Err(anyhow!(
                        "Please provide either --from-share-id, --from-vault-name, or set a default vault with 'pass-cli settings set default-vault'"
                    ));
                }
            } else {
                (from_share_id, from_vault_name)
            };

            let query = r#move::MoveItemQuery::new(
                from_share_id,
                from_vault_name,
                item_id,
                item_title,
                to_share_id,
                to_vault_name,
            )?;
            r#move::run(client, query).await
        }
        ItemCommands::Attachment { attachment_command } => {
            attachment::run(attachment_command, client).await
        }
        ItemCommands::Alias { alias_command } => alias::run(alias_command, client).await,
        ItemCommands::Member(member_cmd) => member::run(client, member_cmd).await,
        ItemCommands::Totp {
            share_id,
            vault_name,
            item_id,
            item_title,
            uri,
            field,
            output,
        } => {
            // Apply default vault if both are None and URI is not provided
            let (share_id, vault_name) =
                if share_id.is_none() && vault_name.is_none() && uri.is_none() {
                    if let Some(default_share_id) =
                        settings_helper::get_default_share_id(&client).await?
                    {
                        (Some(default_share_id.to_string()), None)
                    } else {
                        (None, None)
                    }
                } else {
                    (share_id, vault_name)
                };

            let query =
                totp::ViewTotpQuery::new(share_id, vault_name, item_id, item_title, field, uri)?;
            totp::run(client, query, output).await
        }
        ItemCommands::Trash {
            share_id,
            vault_name,
            item_id,
            item_title,
        } => {
            // Apply default vault if both are None
            let (share_id, vault_name) = if share_id.is_none() && vault_name.is_none() {
                if let Some(default_share_id) =
                    settings_helper::get_default_share_id(&client).await?
                {
                    (Some(default_share_id.to_string()), None)
                } else {
                    return Err(anyhow!(
                        "Please provide either --share-id, --vault-name, or set a default vault with 'pass-cli settings set default-vault'"
                    ));
                }
            } else {
                (share_id, vault_name)
            };

            let query = trash::TrashItemQuery::new(share_id, vault_name, item_id, item_title)?;
            trash::run(client, query).await
        }
        ItemCommands::Untrash {
            share_id,
            vault_name,
            item_id,
            item_title,
        } => {
            // Apply default vault if both are None
            let (share_id, vault_name) = if share_id.is_none() && vault_name.is_none() {
                if let Some(default_share_id) =
                    settings_helper::get_default_share_id(&client).await?
                {
                    (Some(default_share_id.to_string()), None)
                } else {
                    return Err(anyhow!(
                        "Please provide either --share-id, --vault-name, or set a default vault with 'pass-cli settings set default-vault'"
                    ));
                }
            } else {
                (share_id, vault_name)
            };

            let query = untrash::UntrashItemQuery::new(share_id, vault_name, item_id, item_title)?;
            untrash::run(client, query).await
        }
        ItemCommands::Update {
            share_id,
            vault_name,
            item_id,
            item_title,
            fields,
        } => {
            // Apply default vault if both are None
            let share_query = match (&share_id, &vault_name) {
                (None, None) => {
                    if let Some(default_share_id) =
                        settings_helper::get_default_share_id(&client).await?
                    {
                        common::ShareQuery::ShareId(default_share_id)
                    } else {
                        return Err(anyhow!(
                            "Please provide either --share-id, --vault-name, or set a default vault with 'pass-cli settings set default-vault'"
                        ));
                    }
                }
                _ => common::ShareQuery::new(share_id, vault_name)?,
            };
            let item_query = common::ItemQuery::new(item_id, item_title)?;
            update::run(client, share_query, item_query, fields).await
        }
    }
}
