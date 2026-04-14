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

use anyhow::Result;
use pass_auth::CredentialProvider;

pub const PERSONAL_ACCESS_TOKEN_ENV_VAR: &str = "PROTON_PASS_PERSONAL_ACCESS_TOKEN";

pub struct CliCredentialProvider;

#[async_trait::async_trait]
impl CredentialProvider for CliCredentialProvider {
    async fn get_username(&self) -> Result<String> {
        crate::client::get_username()
    }

    async fn get_password(&self) -> Result<String> {
        crate::client::get_password()
    }

    async fn get_totp(&self) -> Result<String> {
        crate::client::get_totp()
    }

    async fn get_extra_password(&self) -> Result<String> {
        crate::client::get_extra_password()
    }

    async fn get_personal_access_token(&self) -> Result<String> {
        std::env::var(PERSONAL_ACCESS_TOKEN_ENV_VAR)
            .map_err(|_| anyhow::anyhow!(
                "Personal access token token not found. Set {PERSONAL_ACCESS_TOKEN_ENV_VAR} environment variable"
            ))
    }
}
