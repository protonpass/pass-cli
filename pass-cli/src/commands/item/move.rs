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
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};

pub struct MoveItemQuery {
    from_share_query: ShareQuery,
    item_query: ItemQuery,
    to_share_query: ShareQuery,
}

impl MoveItemQuery {
    pub fn new(
        from_share_id: Option<String>,
        from_vault: Option<String>,
        item_id: Option<String>,
        item_title: Option<String>,
        to_share_id: Option<String>,
        to_vault_name: Option<String>,
    ) -> Result<Self> {
        let from_share_query = ShareQuery::new_with_arg_name(
            from_share_id,
            from_vault,
            "--from-share-id",
            "--from-vault",
        )?;
        let item_query = ItemQuery::new(item_id, item_title)?;

        let to_share_query = ShareQuery::new_with_arg_name(
            to_share_id,
            to_vault_name,
            "--to-share-id",
            "--to-vault-name",
        )?;

        Ok(Self {
            from_share_query,
            item_query,
            to_share_query,
        })
    }
}

pub async fn run(client: PassClient, query: MoveItemQuery) -> Result<()> {
    let from_share_id = query.from_share_query.share_id(&client).await?;
    let item_id = query.item_query.item_id(&from_share_id, &client).await?;
    let to_share_id = query.to_share_query.share_id(&client).await?;

    let new_item_id = client
        .move_item(&from_share_id, &item_id, &to_share_id)
        .await
        .context("Error moving item")?;

    println!("{new_item_id}");
    Ok(())
}
