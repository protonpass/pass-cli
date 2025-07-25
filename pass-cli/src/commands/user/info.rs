use crate::commands::OutputFormat;
use anyhow::{Context, Result};
use chrono::TimeZone;
use pass::PassClient;

#[derive(serde::Serialize)]
struct UserInfoJsonOutput<'a> {
    pub email: &'a str,
    pub plan: &'a str,
    pub subscription_end: Option<u64>,
    pub vault_limit: Option<u16>,
    pub alias_limit: Option<u16>,
    pub totp_limit: Option<u16>,
    pub storage_used: u64,
    pub storage_quota: u64,
}

pub async fn run(client: PassClient, output_format: OutputFormat) -> Result<()> {
    let addresses = client.get_addresses().await?;
    let primary_address = addresses.first().ok_or_else(|| {
        anyhow::anyhow!("No addresses found. Please add an address to your account.")
    })?;
    let user_info = client.get_user_access().await?;

    match output_format {
        OutputFormat::Human => {
            println!("User: {}", &primary_address.email);
            println!("Plan: {}", &user_info.plan.display_name);
            if let Some(subscription_end) = user_info.plan.subscription_end {
                let end_date = chrono::Utc.timestamp_opt(subscription_end as i64, 0);
                println!(
                    "Subscription Ends: {}",
                    end_date.unwrap().format("%Y-%m-%d %H:%M:%S")
                );
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
        }
        OutputFormat::Json => {
            let out = UserInfoJsonOutput {
                email: &primary_address.email,
                plan: &user_info.plan.display_name,
                subscription_end: user_info.plan.subscription_end,
                vault_limit: user_info.plan.vault_limit,
                alias_limit: user_info.plan.alias_limit,
                totp_limit: user_info.plan.totp_limit,
                storage_used: user_info.plan.storage_used,
                storage_quota: user_info.plan.storage_quota,
            };
            let as_json =
                serde_json::to_string_pretty(&out).context("Error serializing user info")?;
            println!("{as_json}");
        }
    }

    Ok(())
}
