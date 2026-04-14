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

use super::{PatExpiration, expiration_to_timestamp};
use crate::commands::OutputFormat;
use crate::commands::settings_helper::get_format;
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use pass::CreatePersonalAccessTokenArgs;

#[derive(Clone, Debug, serde::Serialize)]
struct CreatePersonalAccessTokenResult {
    env_var: String,
    pat_id: String,
}

pub async fn run(
    client: PassClient,
    name: String,
    expiration: PatExpiration,
    format: Option<OutputFormat>,
) -> Result<()> {
    let format = get_format(format, &client).await?;
    let expiration_timestamp = expiration_to_timestamp(&expiration)?;

    let args = CreatePersonalAccessTokenArgs::new(name, expiration_timestamp)?;
    let response = client.create_personal_access_token(args).await?;

    match format {
        OutputFormat::Json => {
            let res = CreatePersonalAccessTokenResult {
                env_var: response.env_var,
                pat_id: response.personal_access_token_id.value().to_string(),
            };
            let serialized = serde_json::to_string_pretty(&res)?;
            println!("{}", serialized);
        }
        OutputFormat::Human => {
            println!("{}", response.env_var);
        }
    }

    Ok(())
}
