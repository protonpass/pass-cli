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
use crate::helpers::PassClientExt;
use anyhow::{Result, anyhow};
use clap::Subcommand;
use pass_db::{Setting, UserSettingModel};

#[derive(Subcommand)]
pub enum UnsetCommands {
    #[command(about = "Unset the default vault")]
    DefaultVault,

    #[command(about = "Unset the default output format")]
    DefaultFormat,
}

pub async fn run(subcommand: UnsetCommands, client: PassClient) -> Result<()> {
    let setting = match subcommand {
        UnsetCommands::DefaultVault => Setting::DefaultShareId,
        UnsetCommands::DefaultFormat => Setting::DefaultFormat,
    };

    let client_features = client.get_cli_client_features()?;
    let db = &client_features.database_manager;
    let conn = db.get_connection().await?;

    let user_id = client_features
        .get_user_id()
        .await
        .ok_or_else(|| anyhow!("No active session"))?;

    let deleted = UserSettingModel::delete(&conn, &user_id, setting).await?;

    if deleted > 0 {
        println!("Setting '{}' has been cleared", setting.key());
    } else {
        println!("Setting '{}' was not set", setting.key());
    }

    Ok(())
}
