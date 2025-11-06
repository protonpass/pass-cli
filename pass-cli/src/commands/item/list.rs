use crate::commands::OutputFormat;
use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_domain::{Item, ItemContent, ItemState, ShareId};
use std::str::FromStr;

#[derive(serde::Serialize)]
struct ItemsList {
    items: Vec<Item>,
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
    output: OutputFormat,
) -> Result<()> {
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
            let list = ItemsList { items };
            let json = serde_json::to_string_pretty(&list).context("Error serializing items")?;
            println!("{json}");
        }
        OutputFormat::Human => {
            for item in items {
                println!(
                    "- [{}]: {} (state={:?})",
                    item.id, item.content.title, item.state
                );
            }
        }
    }

    Ok(())
}
