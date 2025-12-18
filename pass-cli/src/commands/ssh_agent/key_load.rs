use super::VaultQuery;
use super::key_storage::{IdentitySource, SshIdentity};
use anyhow::{Context, Result, anyhow};
use futures::stream::{self, StreamExt};
use pass::PassClient;
use pass_domain::{Item, ItemContent, ItemState};
use ssh_key::private::PrivateKey as SshPrivateKey;
use std::collections::HashSet;

const MAX_PARALLEL_SHARE_FETCHES: usize = 20;

pub struct SshKeyItem {
    pub item: Item,
    pub private_key: String,
}

pub async fn load_ssh_keys_from_vaults(
    client: &PassClient,
    query: VaultQuery,
) -> Result<Vec<SshKeyItem>> {
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

fn extract_ssh_keys(items: Vec<Item>) -> Vec<SshKeyItem> {
    items
        .into_iter()
        .filter_map(|item| match item.state {
            ItemState::Active => match item.content.content {
                ItemContent::SshKey(ref ssh_key) => Some(SshKeyItem {
                    item: item.clone(),
                    private_key: ssh_key.private_key.clone(),
                }),
                _ => None,
            },
            ItemState::Trashed => None,
        })
        .collect()
}

fn find_passphrases_in_extra_fields(item: &Item) -> Vec<String> {
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

// Attempts to reformat a malformed SSH private key where newlines were replaced with spaces.
// This handles a legacy bug where SSH private keys were stored with newlines replaced by spaces,
// turning a properly formatted key into a single line.
fn reformat_malformed_ssh_key(key: &str) -> Option<String> {
    const HEADER: &str = "-----BEGIN OPENSSH PRIVATE KEY-----";
    const FOOTER: &str = "-----END OPENSSH PRIVATE KEY-----";
    const LINE_WIDTH: usize = 70;

    // Check if key contains header and footer
    let header_pos = key.find(HEADER)?;
    let footer_pos = key.find(FOOTER)?;

    if footer_pos <= header_pos {
        return None;
    }

    // Extract the base64 content between header and footer
    let content_start = header_pos + HEADER.len();
    let content = key[content_start..footer_pos].trim();

    // Remove all whitespace from the base64 content
    let base64_content: String = content.chars().filter(|c| !c.is_whitespace()).collect();

    if base64_content.is_empty() {
        return None;
    }

    // Reconstruct the key with proper formatting
    let mut result = String::with_capacity(key.len() + 20);
    result.push_str(HEADER);
    result.push('\n');

    // Wrap base64 content at LINE_WIDTH characters
    for chunk in base64_content.as_bytes().chunks(LINE_WIDTH) {
        result.push_str(std::str::from_utf8(chunk).ok()?);
        result.push('\n');
    }

    result.push_str(FOOTER);
    result.push('\n');

    Some(result)
}

fn load_key(private_key_str: &str) -> Result<SshPrivateKey> {
    match SshPrivateKey::from_openssh(private_key_str) {
        Ok(ssh_key) => Ok(ssh_key),
        Err(original_error) => {
            // Try to reformat the key in case it was stored with newlines replaced by spaces
            if let Some(reformatted_key) = reformat_malformed_ssh_key(private_key_str) {
                debug!("Attempting to load SSH key after reformatting (legacy format detected)");
                match SshPrivateKey::from_openssh(&reformatted_key) {
                    Ok(ssh_key) => {
                        debug!("Successfully loaded SSH key after reformatting");
                        Ok(ssh_key)
                    }
                    Err(_) => {
                        // Return the original error since reformatting didn't help
                        Err(original_error.into())
                    }
                }
            } else {
                Err(original_error.into())
            }
        }
    }
}

pub fn load_and_decrypt_key(item: &Item, private_key_str: &str) -> Result<SshPrivateKey> {
    let private_key = load_key(private_key_str).context(format!(
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

pub async fn fetch_ssh_keys(
    client: &PassClient,
    vault_query: &VaultQuery,
) -> Result<Vec<SshIdentity>> {
    let ssh_key_items = load_ssh_keys_from_vaults(client, vault_query.clone())
        .await
        .context("Failed to load SSH keys from vaults")?;

    if ssh_key_items.is_empty() {
        return Ok(Vec::new());
    }

    let mut identities = Vec::new();

    for ssh_item in ssh_key_items {
        let item = &ssh_item.item;
        match load_and_decrypt_key(item, &ssh_item.private_key) {
            Ok(private_key) => match SshIdentity::new(
                private_key,
                item.content.title.clone(),
                IdentitySource::ProtonPass {
                    share_id: item.share_id.clone(),
                    item_id: item.id.clone(),
                },
            ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // A valid SSH RSA private key for testing (unencrypted, 1024-bit for smaller size)
    const VALID_SSH_KEY: &str = r#"-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAlwAAAAdzc2gtcn
NhAAAAAwEAAQAAAIEAwUYneWAiJGADOGPA17314hqc9MN4Ci2YWrZGyesGXp1aIKtWhf4o
pB+/di3+3O+b2lMbq2I+Uk0U5sB/I6UQzVuWt1rK6IBmhtt5UrUpNwOwKQCzuSN0ZsgDje
FavGqKBUgfOcSnXOimAR+FbwQ73ga4CHp1wQVvTj99n7gODW0AAAIA8yP0bvMj9G4AAAAH
c3NoLXJzYQAAAIEAwUYneWAiJGADOGPA17314hqc9MN4Ci2YWrZGyesGXp1aIKtWhf4opB
+/di3+3O+b2lMbq2I+Uk0U5sB/I6UQzVuWt1rK6IBmhtt5UrUpNwOwKQCzuSN0ZsgDjeFa
vGqKBUgfOcSnXOimAR+FbwQ73ga4CHp1wQVvTj99n7gODW0AAAADAQABAAAAgQCUA7QLYj
IDhXwx3UM8dgAujo8Ra/ksYkrBfcKstE8Gep8hUdZLe5+IQcARM5xxexbylp8kG3L6+Ik/
RsCXfbxlEPZ9SwoYUPvhLfJ0FyI1DXmUeg9TLOSjRgRY8P6l+GEiwR/Ghr04aD6TiljoJP
Q+plAfp1bcMq2FNJVYSkXuwQAAAEBum4W1xKa1sAm2GIt2+wHUTSkOjT2j0KZidYzKPMoP
um7h7P7mk8jP64KBu1XE2bIJ8Hs1MAlMQztcgwBcUcGlAAAAQQD/l+lWVhLFgYcLVPegrV
gNQJF2emJTcTvEPWcm6E2TzOMD27ILGq73DMquS1IrHw+PRNQQkKGFfvgbuz9y5e8pAAAA
QQDBlN0WR/PmWc117JXfMOvo3XMFd1NiQhvaNLezkDH7s9MVVd0TbxRwM3TZRv6NF0eKzt
pRr1L3pDxhi5yCqKilAAAABm5vbmFtZQECAwQ=
-----END OPENSSH PRIVATE KEY-----
"#;

    // The same key but with newlines replaced by spaces (the legacy bug format)
    const MALFORMED_SSH_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY----- b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAlwAAAAdzc2gtcn NhAAAAAwEAAQAAAIEAwUYneWAiJGADOGPA17314hqc9MN4Ci2YWrZGyesGXp1aIKtWhf4o pB+/di3+3O+b2lMbq2I+Uk0U5sB/I6UQzVuWt1rK6IBmhtt5UrUpNwOwKQCzuSN0ZsgDje FavGqKBUgfOcSnXOimAR+FbwQ73ga4CHp1wQVvTj99n7gODW0AAAIA8yP0bvMj9G4AAAAH c3NoLXJzYQAAAIEAwUYneWAiJGADOGPA17314hqc9MN4Ci2YWrZGyesGXp1aIKtWhf4opB +/di3+3O+b2lMbq2I+Uk0U5sB/I6UQzVuWt1rK6IBmhtt5UrUpNwOwKQCzuSN0ZsgDjeFa vGqKBUgfOcSnXOimAR+FbwQ73ga4CHp1wQVvTj99n7gODW0AAAADAQABAAAAgQCUA7QLYj IDhXwx3UM8dgAujo8Ra/ksYkrBfcKstE8Gep8hUdZLe5+IQcARM5xxexbylp8kG3L6+Ik/ RsCXfbxlEPZ9SwoYUPvhLfJ0FyI1DXmUeg9TLOSjRgRY8P6l+GEiwR/Ghr04aD6TiljoJP Q+plAfp1bcMq2FNJVYSkXuwQAAAEBum4W1xKa1sAm2GIt2+wHUTSkOjT2j0KZidYzKPMoP um7h7P7mk8jP64KBu1XE2bIJ8Hs1MAlMQztcgwBcUcGlAAAAQQD/l+lWVhLFgYcLVPegrV gNQJF2emJTcTvEPWcm6E2TzOMD27ILGq73DMquS1IrHw+PRNQQkKGFfvgbuz9y5e8pAAAA QQDBlN0WR/PmWc117JXfMOvo3XMFd1NiQhvaNLezkDH7s9MVVd0TbxRwM3TZRv6NF0eKzt pRr1L3pDxhi5yCqKilAAAABm5vbmFtZQECAwQ= -----END OPENSSH PRIVATE KEY-----";

    #[test]
    fn test_reformat_malformed_ssh_key_restores_valid_format() {
        let reformatted = reformat_malformed_ssh_key(MALFORMED_SSH_KEY);
        assert!(
            reformatted.is_some(),
            "Should successfully reformat the key"
        );

        let reformatted = reformatted.unwrap();

        // Check structure
        assert!(
            reformatted.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----\n"),
            "Should have proper header with newline"
        );
        assert!(
            reformatted.ends_with("-----END OPENSSH PRIVATE KEY-----\n"),
            "Should have proper footer with newline"
        );

        // Check line lengths (base64 lines should be max 70 chars)
        for line in reformatted.lines() {
            if !line.starts_with("-----") {
                assert!(
                    line.len() <= 70,
                    "Base64 lines should be at most 70 characters, got {}",
                    line.len()
                );
            }
        }
    }

    #[test]
    fn test_reformat_malformed_ssh_key_with_valid_key() {
        // Reformatting a valid key should also work
        let reformatted = reformat_malformed_ssh_key(VALID_SSH_KEY);
        assert!(reformatted.is_some(), "Should handle valid key format too");
    }

    #[test]
    fn test_reformat_malformed_ssh_key_returns_none_for_invalid() {
        assert!(reformat_malformed_ssh_key("not a key").is_none());
        assert!(reformat_malformed_ssh_key("").is_none());
        assert!(reformat_malformed_ssh_key("-----BEGIN OPENSSH PRIVATE KEY-----").is_none());
        assert!(reformat_malformed_ssh_key("-----END OPENSSH PRIVATE KEY-----").is_none());
        // Footer before header
        assert!(
            reformat_malformed_ssh_key(
                "-----END OPENSSH PRIVATE KEY----- -----BEGIN OPENSSH PRIVATE KEY-----"
            )
            .is_none()
        );
    }

    #[test]
    fn test_reformat_malformed_ssh_key_empty_content() {
        let empty_content = "-----BEGIN OPENSSH PRIVATE KEY----- -----END OPENSSH PRIVATE KEY-----";
        assert!(
            reformat_malformed_ssh_key(empty_content).is_none(),
            "Should return None for empty base64 content"
        );
    }

    #[test]
    fn test_load_key_with_valid_key() {
        let result = load_key(VALID_SSH_KEY);
        assert!(
            result.is_ok(),
            "Should load valid SSH key: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_load_key_with_malformed_key() {
        let result = load_key(MALFORMED_SSH_KEY);
        assert!(
            result.is_ok(),
            "Should load malformed SSH key after reformatting: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_load_key_invalid_key_fails() {
        let result = load_key("not a valid key at all");
        assert!(result.is_err(), "Should fail for completely invalid input");
    }

    #[test]
    fn test_load_key_produces_same_key_for_both_formats() {
        let valid_key = load_key(VALID_SSH_KEY).expect("Should load valid key");
        let reformatted_key = load_key(MALFORMED_SSH_KEY).expect("Should load malformed key");

        // Compare the public keys to ensure they represent the same key
        assert_eq!(
            valid_key.public_key().to_bytes(),
            reformatted_key.public_key().to_bytes(),
            "Both formats should produce the same key"
        );
    }
}
