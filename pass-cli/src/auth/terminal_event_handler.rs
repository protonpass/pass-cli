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
use pass_auth::AuthEventHandler;

pub struct TerminalEventHandler;

#[async_trait::async_trait]
impl AuthEventHandler for TerminalEventHandler {
    async fn on_web_login_url_generated(&self, url: &str) -> Result<()> {
        println!("\nPlease open the following URL in your browser to complete authentication:");
        println!("\n{}\n", url);
        println!("Waiting for authentication to complete...");
        Ok(())
    }

    async fn on_poll_progress(&self, attempt: u32, max_attempts: u32) -> Result<()> {
        info!("Polling attempt {}/{}", attempt, max_attempts);
        Ok(())
    }

    async fn on_auth_success(&self, message: &str) -> Result<()> {
        println!("{message}");
        Ok(())
    }

    async fn on_extra_password_required(&self) -> Result<()> {
        info!("Account needs Pass extra password");
        Ok(())
    }

    async fn on_info(&self, message: &str) -> Result<()> {
        println!("{}", message);
        Ok(())
    }

    async fn on_warning(&self, message: &str) -> Result<()> {
        println!("{}", message);
        Ok(())
    }

    async fn on_error(&self, message: &str) -> Result<()> {
        eprintln!("{}", message);
        eprintln!("Make sure the password you entered is the right one.");
        Ok(())
    }
}
