/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::commands::{OutputFormat, settings_helper};
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::{FolderId, Item, ItemContent, ItemFlag, ItemId, ItemState, ShareId, VaultId};
use std::str::FromStr;

#[derive(serde::Serialize)]
struct ItemsList<T: serde::Serialize> {
    items: Vec<T>,
    #[cfg(feature = "internal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    folders: Option<Vec<FolderInfo>>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ItemType {
    Note,
    Login,
    Alias,
    CreditCard,
    Identity,
    SshKey,
    Wifi,
    Custom,
}

impl From<&ItemContent> for ItemType {
    fn from(content: &ItemContent) -> Self {
        match content {
            ItemContent::Note(_) => ItemType::Note,
            ItemContent::Login(_) => ItemType::Login,
            ItemContent::Alias(_) => ItemType::Alias,
            ItemContent::CreditCard(_) => ItemType::CreditCard,
            ItemContent::Identity(_) => ItemType::Identity,
            ItemContent::SshKey(_) => ItemType::SshKey,
            ItemContent::Wifi(_) => ItemType::Wifi,
            ItemContent::Custom(_) => ItemType::Custom,
        }
    }
}

// Fields here must never carry user-provided secret material (no content, note, extra_fields).
#[derive(serde::Serialize)]
struct ItemSummary {
    id: ItemId,
    share_id: ShareId,
    vault_id: VaultId,
    state: ItemState,
    flags: Vec<ItemFlag>,
    create_time: jiff::civil::DateTime,
    modify_time: jiff::civil::DateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    folder_id: Option<FolderId>,
    title: String,
    item_type: ItemType,
}

impl From<&Item> for ItemSummary {
    fn from(item: &Item) -> Self {
        ItemSummary {
            id: item.id.clone(),
            share_id: item.share_id.clone(),
            vault_id: item.vault_id.clone(),
            state: item.state,
            flags: item.flags.clone(),
            create_time: item.create_time,
            modify_time: item.modify_time,
            folder_id: item.folder_id.clone(),
            title: item.content.title.clone(),
            item_type: ItemType::from(&item.content.content),
        }
    }
}

#[cfg(feature = "internal")]
#[derive(serde::Serialize)]
struct FolderInfo {
    folder_id: String,
    folder_name: String,
    parent_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    Note,
    Login,
    Alias,
    CreditCard,
    Identity,
    SshKey,
    Wifi,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterState {
    Active,
    Trashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    AlphabeticAsc,
    AlphabeticDesc,
    CreatedAsc,
    CreatedDesc,
}

impl FilterType {
    #[allow(clippy::match_like_matches_macro)]
    pub fn matches(&self, content: &ItemContent) -> bool {
        match (self, content) {
            (FilterType::Note, ItemContent::Note(_)) => true,
            (FilterType::Login, ItemContent::Login(_)) => true,
            (FilterType::Alias, ItemContent::Alias(_)) => true,
            (FilterType::CreditCard, ItemContent::CreditCard(_)) => true,
            (FilterType::Identity, ItemContent::Identity(_)) => true,
            (FilterType::SshKey, ItemContent::SshKey(_)) => true,
            (FilterType::Wifi, ItemContent::Wifi(_)) => true,
            (FilterType::Custom, ItemContent::Custom(_)) => true,
            _ => false,
        }
    }
}

impl FromStr for FilterType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "note" => Ok(FilterType::Note),
            "login" => Ok(FilterType::Login),
            "alias" => Ok(FilterType::Alias),
            "credit-card" => Ok(FilterType::CreditCard),
            "identity" => Ok(FilterType::Identity),
            "ssh-key" => Ok(FilterType::SshKey),
            "wifi" => Ok(FilterType::Wifi),
            "custom" => Ok(FilterType::Custom),
            _ => Err(anyhow!(
                "Invalid filter type '{}'. Valid types are: note, login, alias, credit-card, identity, ssh-key, wifi, custom",
                s
            )),
        }
    }
}

impl FilterState {
    pub fn matches(&self, state: ItemState) -> bool {
        match self {
            FilterState::Active => state == ItemState::Active,
            FilterState::Trashed => state == ItemState::Trashed,
        }
    }
}

impl FromStr for FilterState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "active" => Ok(FilterState::Active),
            "trashed" => Ok(FilterState::Trashed),
            _ => Err(anyhow!(
                "Invalid filter state '{}'. Valid states are: active, trashed",
                s
            )),
        }
    }
}

impl SortBy {
    pub fn sort_items(&self, items: &mut [Item]) {
        match self {
            SortBy::AlphabeticAsc => {
                items.sort_by(|a, b| {
                    a.content
                        .title
                        .to_lowercase()
                        .cmp(&b.content.title.to_lowercase())
                });
            }
            SortBy::AlphabeticDesc => {
                items.sort_by(|a, b| {
                    b.content
                        .title
                        .to_lowercase()
                        .cmp(&a.content.title.to_lowercase())
                });
            }
            SortBy::CreatedAsc => {
                items.sort_by_key(|a| a.create_time);
            }
            SortBy::CreatedDesc => {
                items.sort_by_key(|a| std::cmp::Reverse(a.create_time));
            }
        }
    }
}

impl FromStr for SortBy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "alphabetic-asc" => Ok(SortBy::AlphabeticAsc),
            "alphabetic-desc" => Ok(SortBy::AlphabeticDesc),
            "created-asc" => Ok(SortBy::CreatedAsc),
            "created-desc" => Ok(SortBy::CreatedDesc),
            _ => Err(anyhow!(
                "Invalid sort type '{}'. Valid types are: alphabetic-asc, alphabetic-desc, created-asc, created-desc",
                s
            )),
        }
    }
}

pub enum ListItemsQuery {
    ShareId(ShareId),
    VaultName(String),
}

impl ListItemsQuery {
    pub fn new(share_id: Option<String>, name: Option<String>) -> Result<Self> {
        match (share_id, name) {
            (Some(share_id), None) => Ok(Self::ShareId(ShareId::new(share_id))),
            (None, Some(vault_name)) => Ok(Self::VaultName(vault_name)),

            _ => Err(anyhow!("Please provide either share-id or vault name")),
        }
    }
}

pub async fn run(
    client: PassClient,
    query: ListItemsQuery,
    filter_type: Option<FilterType>,
    filter_state: Option<FilterState>,
    sort_by: Option<SortBy>,
    output: Option<OutputFormat>,
    show_secrets: bool,
) -> Result<()> {
    // Resolve output format from settings if not provided
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

    if show_secrets && client.is_agent_session() {
        return Err(anyhow!(
            "--show-secrets is not allowed with an agent session"
        ));
    }
    if show_secrets && !matches!(output, OutputFormat::Json) {
        return Err(anyhow!("--show-secrets requires --output json"));
    }

    let share_id = match query {
        ListItemsQuery::ShareId(id) => id,
        ListItemsQuery::VaultName(vault) => {
            let vault = client
                .find_vault(&vault)
                .await
                .context("Error finding vault")?;
            vault.share_id
        }
    };
    let mut items = client
        .list_items(&share_id)
        .await
        .context("Error listing items")?;

    if let Some(filter) = filter_type {
        items.retain(|item| filter.matches(&item.content.content));
    }

    if let Some(filter) = filter_state {
        items.retain(|item| filter.matches(item.state));
    }

    if let Some(sort) = sort_by {
        sort.sort_items(&mut items);
    }

    match output {
        OutputFormat::Json => {
            #[cfg(feature = "internal")]
            let folders = match client.list_folders(&share_id).await {
                Ok(folders) => Some(
                    folders
                        .into_iter()
                        .map(|f| FolderInfo {
                            folder_id: f.id.to_string(),
                            folder_name: f.content.name,
                            parent_id: f.parent_folder_id.map(|id| id.to_string()),
                        })
                        .collect(),
                ),
                Err(e) => {
                    error!("Error listing folders: {e:#}");
                    None
                }
            };

            let json = if show_secrets {
                let list = ItemsList {
                    items,
                    #[cfg(feature = "internal")]
                    folders,
                };
                serde_json::to_string_pretty(&list).context("Error serializing items")?
            } else {
                let summaries: Vec<ItemSummary> = items.iter().map(ItemSummary::from).collect();
                let list = ItemsList {
                    items: summaries,
                    #[cfg(feature = "internal")]
                    folders,
                };
                serde_json::to_string_pretty(&list).context("Error serializing items")?
            };
            println!("{json}");
        }
        OutputFormat::Human => {
            for item in items {
                let suffix = match &item.folder_id {
                    Some(folder_id) => match client.get_folder_name(&share_id, folder_id).await {
                        Ok(name) => format!(" [folder: {}]", name),
                        Err(e) => {
                            error!("Error getting folder name: {e:#}");
                            String::new()
                        }
                    },
                    None => String::new(),
                };
                println!(
                    "- [{}]: {}{} (state={:?})",
                    item.id, item.content.title, suffix, item.state
                );
            }

            #[cfg(feature = "internal")]
            {
                internal::display_folder_tree(&client, &share_id).await;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pass_domain::{
        ItemContent, ItemData, ItemId, ItemState, LoginItem, NoteItem, ShareId, VaultId,
    };

    fn make_item(content: ItemContent) -> Item {
        Item {
            id: ItemId::new("item-1".to_string()),
            share_id: ShareId::new("share-1".to_string()),
            vault_id: VaultId::new("vault-1".to_string()),
            content: ItemData {
                title: "My Item".to_string(),
                note: String::new(),
                item_uuid: "uuid-1".to_string(),
                content,
                extra_fields: vec![],
                platform_specific: None,
            },
            state: ItemState::Active,
            flags: vec![],
            create_time: jiff::civil::DateTime::constant(2026, 1, 1, 0, 0, 0, 0),
            modify_time: jiff::civil::DateTime::constant(2026, 1, 1, 0, 0, 0, 0),
            folder_id: None,
        }
    }

    #[test]
    fn item_type_from_note() {
        let item = make_item(ItemContent::Note(NoteItem));
        let summary = ItemSummary::from(&item);
        assert!(matches!(summary.item_type, ItemType::Note));
        assert_eq!(summary.title, "My Item");
    }

    #[test]
    fn item_type_from_login() {
        let item = make_item(ItemContent::Login(LoginItem {
            email: "a@b.com".to_string(),
            username: "user".to_string(),
            password: "secret".to_string(),
            urls: vec![],
            totp_uri: String::new(),
            passkeys: vec![],
        }));
        let summary = ItemSummary::from(&item);
        assert!(matches!(summary.item_type, ItemType::Login));
    }

    #[test]
    fn summary_has_no_secrets_field() {
        // ItemSummary must NOT contain a `content` field (which would hold secrets).
        // We verify this by checking the serialized JSON keys.
        let item = make_item(ItemContent::Note(NoteItem));
        let summary = ItemSummary::from(&item);
        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("\"content\""));
        assert!(!json.contains("\"password\""));
        assert!(!json.contains("\"note\":"));
        assert!(!json.contains("\"extra_fields\""));
        assert!(json.contains("\"title\""));
        assert!(json.contains("\"item_type\""));
    }
}

#[cfg(feature = "internal")]
mod internal {
    use super::*;
    use pass_domain::Folder;
    use std::collections::HashMap;

    pub async fn display_folder_tree(client: &PassClient, share_id: &ShareId) {
        match client.list_folders(share_id).await {
            Ok(folders) => {
                if !folders.is_empty() {
                    print_folder_tree(&folders);
                }
            }
            Err(e) => {
                error!("Error listing folders: {e:#}");
            }
        }
    }

    fn print_folder_tree(folders: &[Folder]) {
        // Build a map of parent_id -> children
        let mut children_map: HashMap<Option<String>, Vec<&Folder>> = HashMap::new();

        for folder in folders {
            let parent_key = folder.parent_folder_id.as_ref().map(|id| id.to_string());
            children_map.entry(parent_key).or_default().push(folder);
        }
        println!("Folders:");

        // Print root folders (those with no parent)
        if let Some(root_folders) = children_map.get(&None) {
            for (i, folder) in root_folders.iter().enumerate() {
                let is_last = i == root_folders.len() - 1;
                print_folder_node(folder, "", is_last, &children_map);
            }
        }
    }

    fn print_folder_node(
        folder: &Folder,
        prefix: &str,
        is_last: bool,
        children_map: &HashMap<Option<String>, Vec<&Folder>>,
    ) {
        // Print current folder
        let branch = if is_last { "└── " } else { "├── " };
        println!("{}{}{}", prefix, branch, folder.content.name);

        // Print children
        let folder_key = Some(folder.id.to_string());
        if let Some(children) = children_map.get(&folder_key) {
            let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

            for (i, child) in children.iter().enumerate() {
                let is_last_child = i == children.len() - 1;
                print_folder_node(child, &child_prefix, is_last_child, children_map);
            }
        }
    }
}
