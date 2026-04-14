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
use crate::helpers::CliPassClient as PassClient;
use crate::helpers::PassClientExt;
use anyhow::{Context, Result};
use pass_db::{Setting, UserSettingModel};
use pass_domain::ShareId;

pub async fn get_default_share_id(client: &PassClient) -> Result<Option<ShareId>> {
    let setting = get_setting(client, Setting::DefaultShareId).await?;
    Ok(setting.map(ShareId::new))
}

pub async fn get_default_format(client: &PassClient) -> Result<Option<OutputFormat>> {
    let setting = get_setting(client, Setting::DefaultFormat).await?;
    match setting {
        Some(format_str) if format_str == "json" => Ok(Some(OutputFormat::Json)),
        Some(format_str) if format_str == "human" => Ok(Some(OutputFormat::Human)),
        _ => Ok(None),
    }
}

pub async fn get_format(format: Option<OutputFormat>, client: &PassClient) -> Result<OutputFormat> {
    match format {
        Some(o) => Ok(o),
        None => {
            let default = get_default_format(client)
                .await
                .context("could not get default format")?;
            Ok(default.unwrap_or(OutputFormat::Human))
        }
    }
}

async fn get_setting(client: &PassClient, setting: Setting) -> Result<Option<String>> {
    let client_features = client
        .get_cli_client_features()
        .context("Error getting cli client features")?;
    let db = &client_features.database_manager;
    let conn = db
        .get_connection()
        .await
        .context("Error getting database connection")?;

    let user_id = match client_features.get_user_id().await {
        Some(id) => id,
        None => return Ok(None),
    };

    let setting_model = UserSettingModel::get(&conn, &user_id, setting)
        .await
        .context("Error getting user setting")?;
    Ok(setting_model.and_then(|s| s.setting_value))
}
