use super::VaultQuery;
use super::key_storage::{Identity, KeyStorage};
use anyhow::{Context, Result, anyhow};
use futures::stream::{self, StreamExt};
use pass::PassClient;
use pass_domain::{Item, ItemContent};
use ssh_key::private::PrivateKey as SshPrivateKey;
use std::collections::HashSet;

const MAX_PARALLEL_SHARE_FETCHES: usize = 20;

pub async fn load_ssh_keys_from_vaults(
    client: &PassClient,
    query: VaultQuery,
) -> Result<Vec<(Item, String, String)>> {
    let mut all_keys = Vec::new();

    match query {
        VaultQuery::ShareId(share_id) => {
            let items = client
                .list_items(&share_id)
                .await
                .context("Error listing items")?;
            all_keys.extend(extract_ssh_keys(items));
        }
        VaultQuery::VaultName(vault_name) => {
            let vault = client
                .find_vault(&vault_name)
                .await
                .context("Error finding vault")?;
            let items = client
                .list_items(&vault.share_id)
                .await
                .context("Error listing items")?;
            all_keys.extend(extract_ssh_keys(items));
        }
        VaultQuery::All => {
            let shares = client.list_shares().await.context("Error listing shares")?;

            // Fetch all items from all shares in parallel with limited concurrency
            let results: Vec<_> = stream::iter(shares.iter())
                .map(|share| async move {
                    let items = client.list_items(&share.id).await;
                    (share, items)
                })
                .buffer_unordered(MAX_PARALLEL_SHARE_FETCHES)
                .collect()
                .await;

            let mut all_items = Vec::new();
            for (share, result) in results {
                match result {
                    Ok(items) => all_items.extend(items),
                    Err(e) => eprintln!("Error listing items for share {}: {}", share.id, e),
                }
            }

            all_keys.extend(extract_ssh_keys(all_items));
        }
    }

    Ok(all_keys)
}

pub fn extract_ssh_keys(items: Vec<Item>) -> Vec<(Item, String, String)> {
    items
        .into_iter()
        .filter_map(|item| {
            if let ItemContent::SshKey(ref ssh_key) = item.content.content {
                Some((
                    item.clone(),
                    ssh_key.private_key.clone(),
                    ssh_key.public_key.clone(),
                ))
            } else {
                None
            }
        })
        .collect()
}

pub fn find_passphrases_in_extra_fields(item: &Item) -> Vec<String> {
    // Search terms to look for in field names (case-insensitive, partial match)
    let search_terms = [
        "passphrase",
        "password",
        "pass",
        "pwd",
        "key password",
        "ssh pass",
        "ssh password",
        "key pass",
    ];

    let mut res = HashSet::new();
    for extra_field in &item.content.extra_fields {
        let field_name_lower = extra_field.name.to_lowercase();

        // Check if any search term is contained in the field name
        for term in &search_terms {
            if field_name_lower.contains(term) {
                // Extract the content based on field type
                let content = match &extra_field.content {
                    pass_domain::ItemExtraFieldContent::Text(s) => Some(s.clone()),
                    pass_domain::ItemExtraFieldContent::Hidden(s) => Some(s.clone()),
                    pass_domain::ItemExtraFieldContent::Totp(_) => None,
                    pass_domain::ItemExtraFieldContent::Timestamp(_) => None,
                };

                if let Some(passphrase) = content
                    && !passphrase.is_empty()
                {
                    debug!(
                        "Found candidate passphrase in field '{}' for item '{}'",
                        extra_field.name, item.content.title
                    );
                    res.insert(passphrase.to_string());
                }
            }
        }
    }

    // Iterate all extra fields and get the Hidden ones just to have a fallback
    for extra_field in &item.content.extra_fields {
        if let pass_domain::ItemExtraFieldContent::Hidden(ref val) = extra_field.content
            && !val.is_empty()
        {
            debug!(
                "Best effort guess for passphrase in field '{}' for item '{}'",
                extra_field.name, item.content.title
            );
            res.insert(val.to_string());
        }
    }

    res.into_iter().collect()
}

pub fn load_and_decrypt_key(item: &Item, private_key_str: &str) -> Result<SshPrivateKey> {
    let private_key = SshPrivateKey::from_openssh(private_key_str).context(format!(
        "Failed to parse SSH private key for item '{}'",
        item.content.title
    ))?;

    if !private_key.is_encrypted() {
        return Ok(private_key);
    }

    debug!(
        "Key '{}' is encrypted, looking for passphrase",
        item.content.title
    );

    let potential_passphrases = find_passphrases_in_extra_fields(item);
    if !potential_passphrases.is_empty()
        && let Some(passphrase) = potential_passphrases.into_iter().next()
    {
        debug!(
            "Attempting to decrypt key '{}' with found passphrase",
            item.content.title
        );

        let decrypted = private_key.decrypt(passphrase).context(format!(
            "Failed to decrypt SSH key '{}' with provided passphrase",
            item.content.title
        ))?;

        info!("Successfully decrypted SSH key '{}'", item.content.title);
        return Ok(decrypted);
    }

    Err(anyhow!(
        "SSH key '{}' is encrypted but no passphrase found in extra fields. \
        Please add a Hidden field named 'Passphrase' or 'Password' with the key's passphrase.",
        item.content.title
    ))
}

pub async fn load_keys_into_storage(
    client: &PassClient,
    vault_query: &VaultQuery,
) -> Result<Vec<Identity>> {
    let ssh_key_items = load_ssh_keys_from_vaults(client, vault_query.clone())
        .await
        .context("Failed to load SSH keys from vaults")?;

    if ssh_key_items.is_empty() {
        return Ok(Vec::new());
    }

    let mut identities = Vec::new();

    for (item, private_key_str, _public_key_str) in ssh_key_items {
        match load_and_decrypt_key(&item, &private_key_str) {
            Ok(private_key) => match Identity::new(private_key, item.content.title.clone()) {
                Ok(identity) => {
                    identities.push(identity);
                }
                Err(e) => {
                    warn!("Failed to store key '{}': {}", item.content.title, e);
                }
            },
            Err(e) => {
                warn!("Failed to load key '{}': {}", item.content.title, e);
            }
        }
    }

    Ok(identities)
}

pub async fn refresh_keys_periodically(
    client: &PassClient,
    vault_query: &VaultQuery,
    key_storage: &KeyStorage,
    _interval_secs: u64,
) {
    info!("Refreshing SSH keys from Proton Pass...");

    match load_keys_into_storage(client, vault_query).await {
        Ok(identities) => {
            let count = identities.len();
            key_storage.replace_all_identities(identities).await;
            info!("Refreshed {} SSH key(s)", count);
        }
        Err(e) => {
            warn!("Failed to refresh SSH keys: {}. Will retry later.", e);
        }
    }
}
