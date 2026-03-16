use super::key_load::load_and_decrypt_key;
use crate::commands::OutputFormat;
use crate::commands::item::ItemQuery;
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::{Item, ItemContent, ItemState, ShareId};
use serde::Serialize;
use ssh_key::HashAlg;
use ssh_key::private::KeypairData;

#[derive(Debug, Serialize)]
struct DebugReport {
    vault_name: String,
    share_id: String,
    valid_keys: Vec<ValidKeyInfo>,
    invalid_items: Vec<InvalidItemInfo>,
    summary: Summary,
}

#[derive(Debug, Serialize)]
struct ValidKeyInfo {
    title: String,
    algorithm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    key_size: Option<u32>,
    fingerprint: String,
}

#[derive(Debug, Serialize)]
struct InvalidItemInfo {
    title: String,
    item_type: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    valid_keys: usize,
    invalid_items: usize,
    total_items: usize,
}

pub async fn run_debug(
    share_id: Option<String>,
    vault_name: Option<String>,
    item_id: Option<String>,
    item_title: Option<String>,
    output: Option<OutputFormat>,
    client: PassClient,
) -> Result<()> {
    // Require at least one of share_id or vault_name
    if share_id.is_none() && vault_name.is_none() {
        anyhow::bail!("Please provide either --share-id or --vault-name");
    }

    let share_id = resolve_vault(&client, share_id, vault_name).await?;

    // Get the vault to retrieve its name
    let vaults = client
        .list_vaults()
        .await
        .context("Failed to list vaults")?;
    let vault = vaults
        .into_iter()
        .find(|v| v.share_id == share_id)
        .ok_or_else(|| anyhow::anyhow!("Could not find vault with share ID {}", share_id))?;

    info!("Debugging SSH keys in vault: {}", vault.content.name);

    // Check if user wants to debug a specific item
    let items = if item_id.is_some() || item_title.is_some() {
        let item_query = ItemQuery::new(item_id, item_title)?;
        let resolved_item_id = item_query.item_id(&share_id, &client).await?;

        let item_details = client
            .view_item(&share_id, &resolved_item_id)
            .await
            .context("Failed to get item")?;

        vec![item_details.item]
    } else {
        client
            .list_items(&share_id)
            .await
            .context("Failed to list items from vault")?
    };

    let report = categorize_items(items, vault.content.name, share_id.to_string());

    match output.unwrap_or(OutputFormat::Human) {
        OutputFormat::Human => print_human_output(&report),
        OutputFormat::Json => print_json_output(&report)?,
    }

    Ok(())
}

async fn resolve_vault(
    client: &PassClient,
    share_id: Option<String>,
    vault_name: Option<String>,
) -> Result<ShareId> {
    match (share_id, vault_name) {
        (Some(id), None) => Ok(ShareId::new(id)),
        (None, Some(name)) => {
            let vault = client
                .find_vault(&name)
                .await
                .context(format!("Failed to find vault with name '{}'", name))?;
            Ok(vault.share_id)
        }
        _ => unreachable!("Clap validation ensures one is provided"),
    }
}

fn categorize_items(items: Vec<Item>, vault_name: String, share_id: String) -> DebugReport {
    let mut valid_keys = Vec::new();
    let mut invalid_items = Vec::new();

    for item in items {
        match categorize_single_item(&item) {
            ItemCategory::Valid(key_info) => valid_keys.push(key_info),
            ItemCategory::Invalid(item_info) => invalid_items.push(item_info),
        }
    }

    // Calculate lengths before moving
    let valid_count = valid_keys.len();
    let invalid_count = invalid_items.len();
    let total_items = valid_count + invalid_count;

    DebugReport {
        vault_name,
        share_id,
        valid_keys,
        invalid_items,
        summary: Summary {
            valid_keys: valid_count,
            invalid_items: invalid_count,
            total_items,
        },
    }
}

enum ItemCategory {
    Valid(ValidKeyInfo),
    Invalid(InvalidItemInfo),
}

fn categorize_single_item(item: &Item) -> ItemCategory {
    let item_type = match &item.content.content {
        ItemContent::Login(_) => "Login",
        ItemContent::Note(_) => "Note",
        ItemContent::Alias(_) => "Alias",
        ItemContent::CreditCard(_) => "CreditCard",
        ItemContent::Identity(_) => "Identity",
        ItemContent::Custom(_) => "Custom",
        ItemContent::Wifi(_) => "Wifi",
        ItemContent::SshKey(_) => "SshKey",
    };

    // Check if item is trashed
    if item.state == ItemState::Trashed {
        return ItemCategory::Invalid(InvalidItemInfo {
            title: item.content.title.clone(),
            item_type: item_type.to_string(),
            reason: "Item is trashed".to_string(),
        });
    }

    // Check if item is an SSH key type
    let ssh_key = match &item.content.content {
        ItemContent::SshKey(ssh_key) => ssh_key,
        _ => {
            return ItemCategory::Invalid(InvalidItemInfo {
                title: item.content.title.clone(),
                item_type: item_type.to_string(),
                reason: format!("Not an SSH key item (type: {})", item_type),
            });
        }
    };

    // Try to load and decrypt the SSH key
    match load_and_decrypt_key(item, &ssh_key.private_key) {
        Ok(private_key) => match extract_key_details(&private_key, &item.content.title) {
            Ok(key_info) => ItemCategory::Valid(key_info),
            Err(e) => ItemCategory::Invalid(InvalidItemInfo {
                title: item.content.title.clone(),
                item_type: item_type.to_string(),
                reason: format!("Failed to extract key details: {}", e),
            }),
        },
        Err(e) => {
            let reason = format_error_reason(&e);
            ItemCategory::Invalid(InvalidItemInfo {
                title: item.content.title.clone(),
                item_type: item_type.to_string(),
                reason,
            })
        }
    }
}

fn extract_key_details(private_key: &ssh_key::PrivateKey, title: &str) -> Result<ValidKeyInfo> {
    let public_key = private_key.public_key();
    let fingerprint = public_key.fingerprint(HashAlg::Sha256).to_string();

    let (algorithm, key_size) = match private_key.key_data() {
        KeypairData::Rsa(rsa) => {
            let size = rsa.public.n.as_positive_bytes().unwrap().len() * 8;
            ("RSA".to_string(), Some(size as u32))
        }
        KeypairData::Dsa(dsa) => {
            let size = dsa.public.p.as_positive_bytes().unwrap().len() * 8;
            ("DSA".to_string(), Some(size as u32))
        }
        KeypairData::Ecdsa(ecdsa) => {
            let curve = format!("{:?}", ecdsa.curve());
            (format!("ECDSA-{}", curve), None)
        }
        KeypairData::Ed25519(_) => ("Ed25519".to_string(), None),
        KeypairData::SkEcdsaSha2NistP256(_) => ("SK-ECDSA-SHA2-NISTP256".to_string(), None),
        KeypairData::SkEd25519(_) => ("SK-Ed25519".to_string(), None),
        _ => ("Unknown".to_string(), None),
    };

    Ok(ValidKeyInfo {
        title: title.to_string(),
        algorithm,
        key_size,
        fingerprint,
    })
}

fn format_error_reason(error: &anyhow::Error) -> String {
    let error_str = format!("{:#}", error);

    // Check for specific error patterns and format them nicely
    if error_str.contains("is encrypted but no passphrase found") {
        "SSH key is encrypted but no passphrase found in custom fields".to_string()
    } else if error_str.contains("Failed to decrypt SSH key") {
        format!(
            "Failed to decrypt SSH key: {}",
            extract_root_cause(&error_str)
        )
    } else if error_str.contains("Failed to parse SSH private key") {
        format!(
            "Invalid SSH private key format: {}",
            extract_root_cause(&error_str)
        )
    } else if error_str.contains("legacy format detected") {
        "Malformed SSH key format (attempted legacy format recovery)".to_string()
    } else {
        error_str
    }
}

fn extract_root_cause(error_str: &str) -> &str {
    // HACK: Try to get the most specific error message
    error_str.lines().last().unwrap_or(error_str).trim()
}

fn print_human_output(report: &DebugReport) {
    println!("SSH Agent Debug Report");
    println!("Vault: {} ({})", report.vault_name, report.share_id);
    println!();

    if !report.valid_keys.is_empty() {
        println!("✓ Valid SSH Keys ({}):", report.valid_keys.len());
        for key in &report.valid_keys {
            println!("  • {}", key.title);
            if let Some(size) = key.key_size {
                println!("    Algorithm: {}-{}", key.algorithm, size);
            } else {
                println!("    Algorithm: {}", key.algorithm);
            }
            println!("    Fingerprint: {}", key.fingerprint);
            println!();
        }
    }

    if !report.invalid_items.is_empty() {
        println!("✗ Invalid Items ({}):", report.invalid_items.len());
        for item in &report.invalid_items {
            println!("  • {} ({})", item.title, item.item_type);
            println!("    Reason: {}", item.reason);
            println!();
        }
    }

    println!("Summary:");
    println!("  Valid SSH keys: {}", report.summary.valid_keys);
    println!("  Invalid items: {}", report.summary.invalid_items);
    println!("  Total items checked: {}", report.summary.total_items);
}

fn print_json_output(report: &DebugReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report).context("Failed to serialize to JSON")?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pass_domain::{ItemData, ItemId, ItemState, ShareId, SshKeyItem, VaultId};
    use rsa::pkcs8::EncodePrivateKey;

    fn build_ssh_item(title: &str, private_key: String) -> Item {
        Item {
            id: ItemId::new("item-id".to_string()),
            share_id: ShareId::new("share-id".to_string()),
            vault_id: VaultId::new("vault-id".to_string()),
            content: ItemData {
                title: title.to_string(),
                note: String::new(),
                item_uuid: "item-uuid".to_string(),
                content: ItemContent::SshKey(SshKeyItem {
                    private_key,
                    public_key: String::new(),
                    sections: vec![],
                }),
                extra_fields: vec![],
                platform_specific: None,
            },
            state: ItemState::Active,
            flags: vec![],
            create_time: jiff::civil::DateTime::default(),
            modify_time: jiff::civil::DateTime::default(),
            folder_id: None,
        }
    }

    fn generate_pkcs8_rsa_private_key() -> String {
        let mut rng = rand::thread_rng();
        let private_key =
            rsa::RsaPrivateKey::new(&mut rng, 2048).expect("Should generate test RSA key");
        private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("Should encode PKCS#8 private key")
            .to_string()
    }

    #[test]
    fn categorize_single_item_accepts_rsa_pkcs8_private_key() {
        let private_key = generate_pkcs8_rsa_private_key();
        let item = build_ssh_item("Personal", private_key);

        match categorize_single_item(&item) {
            ItemCategory::Valid(key_info) => {
                assert_eq!(key_info.title, "Personal");
                assert_eq!(key_info.algorithm, "RSA");
                assert_eq!(key_info.key_size, Some(2048));
            }
            ItemCategory::Invalid(item_info) => {
                panic!("Expected valid key, got invalid item: {}", item_info.reason)
            }
        }
    }
}
