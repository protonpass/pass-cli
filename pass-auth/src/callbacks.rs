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

#[async_trait::async_trait]
pub trait AuthEventHandler: Send + Sync {
    async fn on_web_login_url_generated(&self, url: &str) -> Result<()>;
    async fn on_poll_progress(&self, attempt: u32, max_attempts: u32) -> Result<()>;
    async fn on_auth_success(&self, message: &str) -> Result<()>;
    async fn on_extra_password_required(&self) -> Result<()>;
    async fn on_info(&self, message: &str) -> Result<()>;
    async fn on_warning(&self, message: &str) -> Result<()>;
    async fn on_error(&self, message: &str) -> Result<()>;
}

#[async_trait::async_trait]
pub trait CredentialProvider: Send + Sync {
    async fn get_username(&self) -> Result<String>;
    async fn get_password(&self) -> Result<String>;
    async fn get_totp(&self) -> Result<String>;
    async fn get_extra_password(&self) -> Result<String>;
    async fn get_personal_access_token(&self) -> Result<String>;
}
