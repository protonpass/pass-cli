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
use anyhow::{Context, Result};
use jiff::Timestamp;
use parking_lot::RwLock;
use pass_auth::PassSessionStore;
use std::sync::Arc;

#[derive(serde::Serialize)]
struct UserInfoJsonOutput {
    pub email: String,
    pub plan: String,
    pub subscription_end: Option<u64>,
    pub vault_limit: Option<u16>,
    pub alias_limit: Option<u16>,
    pub totp_limit: Option<u16>,
    pub storage_used: u64,
    pub storage_quota: u64,
    pub session_has_lock: bool,
}

pub async fn run(
    client: PassClient,
    output_format: OutputFormat,
    store: Arc<RwLock<PassSessionStore>>,
) -> Result<()> {
    let addresses = client.get_addresses().await?;
    let primary_address = addresses.first().ok_or_else(|| {
        anyhow::anyhow!("No addresses found. Please add an address to your account.")
    })?;
    let user_info = client.get_user_access().await?;
    let session_has_lock = store.read().has_session_lock();

    match output_format {
        OutputFormat::Human => {
            println!("User: {}", primary_address.email);
            println!("Plan: {}", user_info.plan.display_name);
            if let Some(subscription_end) = user_info.plan.subscription_end {
                let end_date = Timestamp::from_second(subscription_end as i64)
                    .ok()
                    .map(|ts| ts.to_zoned(jiff::tz::TimeZone::UTC))
                    .map(|zoned| zoned.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "Invalid date".to_string());
                println!("Subscription Ends: {}", end_date);
            }
            if let Some(vault_limit) = user_info.plan.vault_limit {
                println!("Vault limit: {vault_limit}");
            }
            if let Some(alias_limit) = user_info.plan.alias_limit {
                println!("Alias limit: {alias_limit}");
            }
            if let Some(totp_limit) = user_info.plan.totp_limit {
                println!("TOTP limit: {totp_limit}");
            }
            println!(
                "Storage used: {:.2} / {:.2} MiB",
                user_info.plan.storage_used / (1 << 20),
                user_info.plan.storage_quota / (1 << 20)
            );
            if session_has_lock {
                println!("Session has a lock")
            }
        }
        OutputFormat::Json => {
            let out = UserInfoJsonOutput {
                email: primary_address.email.to_string(),
                plan: user_info.plan.display_name,
                subscription_end: user_info.plan.subscription_end,
                vault_limit: user_info.plan.vault_limit,
                alias_limit: user_info.plan.alias_limit,
                totp_limit: user_info.plan.totp_limit,
                storage_used: user_info.plan.storage_used,
                storage_quota: user_info.plan.storage_quota,
                session_has_lock,
            };
            let as_json =
                serde_json::to_string_pretty(&out).context("Error serializing user info")?;
            println!("{as_json}");
        }
    }

    Ok(())
}
