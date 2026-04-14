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

use crate::commands::OutputFormat;
use crate::commands::item::common::ShareQuery;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ItemId;

#[derive(serde::Serialize)]
struct JsonAliasItem {
    id: ItemId,
    alias: String,
}

pub async fn run(
    client: PassClient,
    share_query: ShareQuery,
    prefix: String,
    output: OutputFormat,
) -> Result<()> {
    let share_id = share_query.share_id(&client).await?;
    let res = client
        .create_alias(&share_id, &prefix)
        .await
        .context("Error creating alias")?;

    match output {
        OutputFormat::Human => {
            println!("{}", res.alias);
        }
        OutputFormat::Json => {
            let res = JsonAliasItem {
                id: res.item_id,
                alias: res.alias,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&res).context("Error serializing output")?
            );
        }
    }

    Ok(())
}
