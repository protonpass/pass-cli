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

use super::common::{ItemQuery, ShareQuery};
use crate::commands::item::agent_monitor::send_reason_if_agent;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::EventAction;

pub struct UntrashItemQuery {
    share_query: ShareQuery,
    item_query: ItemQuery,
}

impl UntrashItemQuery {
    pub fn new(
        share_id: Option<String>,
        vault_name: Option<String>,
        item_id: Option<String>,
        item_title: Option<String>,
    ) -> Result<Self> {
        let share_query = ShareQuery::new(share_id, vault_name)?;
        let item_query = ItemQuery::new(item_id, item_title)?;

        Ok(Self {
            share_query,
            item_query,
        })
    }
}

pub async fn run(client: PassClient, query: UntrashItemQuery) -> Result<()> {
    let share_id = query.share_query.share_id(&client).await?;
    let item_id = query.item_query.item_id(&share_id, &client).await?;

    client
        .untrash_item(&share_id, &item_id)
        .await
        .context("Error untrashing item")?;
    send_reason_if_agent(&client, EventAction::ItemUntrash, &share_id, Some(&item_id)).await?;

    println!("Item successfully restored from trash");
    Ok(())
}
