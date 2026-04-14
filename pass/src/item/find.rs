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

use crate::utils::is_id;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use pass_domain::{Item, ItemId, ShareId};

#[derive(Debug)]
pub enum FindItemQuery {
    Name {
        vault_name: String,
        item_name: String,
    },
    Id {
        share_id: ShareId,
        item_id: ItemId,
    },
}

impl FindItemQuery {
    pub fn new(vault: &str, item: &str) -> Self {
        if is_id(vault) && is_id(item) {
            Self::Id {
                share_id: ShareId::new(vault.to_string()),
                item_id: ItemId::new(item.to_string()),
            }
        } else {
            Self::Name {
                vault_name: vault.to_string(),
                item_name: item.to_string(),
            }
        }
    }
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn find_item(&self, query: FindItemQuery) -> Result<Item> {
        match query {
            FindItemQuery::Name {
                vault_name,
                item_name,
            } => self
                .find_item_by_name(&vault_name, &item_name)
                .await
                .context("Error finding item by name"),
            FindItemQuery::Id { share_id, item_id } => {
                let item = self
                    .view_item(&share_id, &item_id)
                    .await
                    .context("Error retrieving item by id")?;
                Ok(item.item)
            }
        }
    }

    async fn find_item_by_name(&self, vault_name: &str, item_name: &str) -> Result<Item> {
        let vault = self
            .find_vault(vault_name)
            .await
            .context("Error finding vault by name")?;
        let items = self
            .list_items(&vault.share_id)
            .await
            .context("Error listing items")?;
        let item = items
            .into_iter()
            .find(|i| i.content.title == item_name)
            .ok_or_else(|| anyhow!("Could not find item with name {}", item_name))?;

        Ok(item)
    }
}
