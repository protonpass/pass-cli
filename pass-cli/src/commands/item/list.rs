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
use pass_domain::{Item, ItemContent, ItemState, ShareId};
use std::str::FromStr;

#[derive(serde::Serialize)]
struct ItemsList {
    items: Vec<Item>,
    #[cfg(feature = "internal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    folders: Option<Vec<FolderInfo>>,
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
                items.sort_by(|a, b| a.create_time.cmp(&b.create_time));
            }
            SortBy::CreatedDesc => {
                items.sort_by(|a, b| b.create_time.cmp(&a.create_time));
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
) -> Result<()> {
    // Resolve output format from settings if not provided
    let output = match output {
        Some(fmt) => fmt,
        None => settings_helper::get_default_format(&client)
            .await?
            .unwrap_or(OutputFormat::Human),
    };

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

            let list = ItemsList {
                items,
                #[cfg(feature = "internal")]
                folders,
            };
            let json = serde_json::to_string_pretty(&list).context("Error serializing items")?;
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
