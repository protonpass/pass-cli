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
use pass_db::{Setting, UserSettingModel};
use std::collections::BTreeMap;

pub async fn run(client: PassClient) -> Result<()> {
    let client_features = client.get_cli_client_features()?;
    let db = &client_features.database_manager;
    let conn = db.get_connection().await?;

    let user_id = client_features
        .get_user_id()
        .await
        .ok_or_else(|| anyhow!("No active session"))?;

    let settings = UserSettingModel::get_by_user_id(&conn, &user_id).await?;

    // Use BTreeMap so the order is deterministic
    let settings_map: BTreeMap<String, Option<String>> = settings
        .into_iter()
        .map(|s| (s.setting_key, s.setting_value))
        .collect();

    println!("Current settings:");
    for setting in Setting::all() {
        match settings_map.get(setting.key()) {
            Some(Some(value)) => println!("  {}: {}", setting.key(), value),
            _ => println!("  {}: {} (default)", setting.key(), setting.default_value()),
        }
    }

    Ok(())
}
