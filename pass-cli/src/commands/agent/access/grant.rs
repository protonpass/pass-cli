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

use crate::commands::Role;
use crate::commands::item::ShareQuery;

use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::{ItemId, ShareRole};

pub async fn run(
    client: PassClient,
    name: String,
    share_id: Option<String>,
    vault_name: Option<String>,
    item_id: Option<String>,
    item_title: Option<String>,
    role: Role,
) -> Result<()> {
    let agent = super::super::find_agent_by_name(&client, &name).await?;

    let share_query = ShareQuery::new(share_id, vault_name)?;
    let resolved_share_id = share_query.share_id(&client).await?;

    let resolved_item_id = if let (Some(_id), Some(_title)) = (&item_id, &item_title) {
        return Err(anyhow!("Cannot specify both --item-id and --item-title"));
    } else if let Some(id) = item_id {
        Some(ItemId::new(id))
    } else if let Some(title) = item_title {
        let items = client
            .list_items(&resolved_share_id)
            .await
            .context("Failed to list items")?;

        let item = items
            .iter()
            .find(|i| i.content.title == title)
            .ok_or_else(|| anyhow!("Item not found: {}", title))?;

        Some(item.id.clone())
    } else {
        None
    };

    client
        .grant_personal_access_token_access(
            &agent.pat_id,
            &resolved_share_id,
            resolved_item_id.as_ref(),
            &ShareRole::from(role),
        )
        .await
        .context("Failed to grant agent access")?;

    println!("Agent access granted successfully");

    Ok(())
}
