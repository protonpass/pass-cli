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

use super::super::PersonalAccessTokenQuery;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ShareId;

pub async fn run(
    client: PassClient,
    query: PersonalAccessTokenQuery,
    share_id: String,
) -> Result<()> {
    let personal_access_token_id = query.resolve(&client).await?;

    client
        .revoke_personal_access_token_access(&personal_access_token_id, &ShareId::new(share_id))
        .await
        .context("Failed to revoke personal access token access")?;

    println!("Personal access token access revoked successfully");

    Ok(())
}
