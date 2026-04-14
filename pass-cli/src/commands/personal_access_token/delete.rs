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

use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result, anyhow};
use pass_domain::PersonalAccessTokenId;

pub async fn run(client: PassClient, personal_access_token_id: String) -> Result<()> {
    if !pass::is_id(&personal_access_token_id) {
        return Err(anyhow!(
            "Not a valid personal access token id: {}",
            personal_access_token_id
        ));
    }

    client
        .delete_personal_access_token(&PersonalAccessTokenId::new(personal_access_token_id))
        .await
        .context("Error deleting personal access token")?;

    println!("Personal access token deleted successfully");

    Ok(())
}
