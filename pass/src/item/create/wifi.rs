use crate::PassClient;
use crate::permission::PermissionAction;
use anyhow::{Context, Result, bail};
use pass_domain::{ItemContent, ItemId, ShareId, WifiItem, WifiSecurity};

#[derive(Clone, Debug)]
pub struct WifiItemCreatePayload {
    pub title: String,
    pub ssid: Option<String>,
    pub password: Option<String>,
    pub security: Option<WifiSecurity>,
    pub note: Option<String>,
}

/// Validates SSID (network name).
/// SSID must not be empty or whitespace-only.
fn validate_ssid(ssid: &str) -> Result<()> {
    if ssid.is_empty() {
        bail!("SSID cannot be empty");
    }

    if ssid.trim().is_empty() {
        bail!("SSID cannot be whitespace only");
    }

    Ok(())
}

impl PassClient {
    pub async fn create_wifi(
        &self,
        share_id: &ShareId,
        payload: WifiItemCreatePayload,
    ) -> Result<ItemId> {
        // Check if user can create WiFi
        self.action_guard(PermissionAction::CreateWifi {
            share_id: share_id.clone(),
        })
        .await?;

        // Validate SSID if provided
        let ssid = payload.ssid.as_deref().unwrap_or("");
        validate_ssid(ssid).context("Invalid SSID")?;

        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                payload.note.as_deref().unwrap_or(""),
                ItemContent::Wifi(WifiItem {
                    ssid: ssid.to_string(),
                    password: payload.password.unwrap_or_default(),
                    security: payload.security.unwrap_or_default(),
                }),
            )
            .await
            .context("Error creating wifi item request")?;

        self.send_create_item_request(share_id, req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use crate::item::create::common::{CreateItemRequest, CreateItemResponse};
    use crate::item::list::ItemRevision;
    use muon::test::server::{HTTP, Server};
    use pass_domain::ItemData;
    use pass_domain::crypto::EncryptionTag;

    // Unit tests for validate_ssid function
    #[test]
    fn test_validate_ssid_valid() {
        // Normal SSIDs
        assert!(validate_ssid("MyNetwork").is_ok());
        assert!(validate_ssid("Home WiFi").is_ok());
        assert!(validate_ssid("Network-5G").is_ok());
        assert!(validate_ssid("Guest_Network").is_ok());

        // With special characters
        assert!(validate_ssid("Network@Home").is_ok());
        assert!(validate_ssid("WiFi #1").is_ok());

        // Unicode SSIDs
        assert!(validate_ssid("网络").is_ok());
        assert!(validate_ssid("Réseau").is_ok());
    }

    #[test]
    fn test_validate_ssid_invalid() {
        // Empty string
        assert!(validate_ssid("").is_err());

        // Whitespace only
        assert!(validate_ssid("   ").is_err());
        assert!(validate_ssid("\t").is_err());
        assert!(validate_ssid("\n").is_err());
    }

    // Integration tests using muon test server
    #[muon::test(scheme(HTTP))]
    async fn test_create_wifi_full_data(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Office WiFi";
        const SSID: &str = "Office-Network-5G";
        const PASSWORD: &str = "secure_password123";
        const NOTE: &str = "Main office network";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        setup(&server); // Set up user data
        setup_paid_user(&server); // Override with paid plan
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let recorder = server.new_recorder();
        server.handler("/pass/v1/share/MyShareID/item", move |_| {
            success(CreateItemResponse {
                item: ItemRevision {
                    item_id: ITEM_ID.to_string(),
                    revision: 0,
                    content_format_version: 0,
                    key_rotation: 0,
                    content: "".to_string(),
                    item_key: None,
                    state: 0,
                    flags: 0,
                    alias_email: None,
                    create_time: 0,
                },
            })
        });

        let item_id = client
            .create_wifi(
                &share_id!(SHARE_ID),
                WifiItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    ssid: Some(SSID.to_string()),
                    password: Some(PASSWORD.to_string()),
                    security: Some(WifiSecurity::WPA2),
                    note: Some(NOTE.to_string()),
                },
            )
            .await
            .expect("Should be able to create the wifi item");

        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Decrypt and verify the item content
        let decoded_encrypted_item_key = crate::utils::b64_decode(&request.item_key).unwrap();
        let decrypted_item_key = pass_domain::crypto::decrypt(
            &decoded_encrypted_item_key,
            &TEST_SHARE_KEY,
            EncryptionTag::ItemKey,
        )
        .expect("Should be able to decrypt item key");

        let decoded_item_content = crate::utils::b64_decode(&request.content).unwrap();
        let decrypted_item_content = pass_domain::crypto::decrypt(
            &decoded_item_content,
            &decrypted_item_key,
            EncryptionTag::ItemContent,
        )
        .expect("Should be able to decrypt item content");

        let parsed_item_content = ItemData::deserialize(&decrypted_item_content)
            .expect("Should be able to deserialize ItemData");

        assert_eq!(ITEM_TITLE, parsed_item_content.title);
        assert_eq!(NOTE, parsed_item_content.note);

        match parsed_item_content.content {
            ItemContent::Wifi(wifi) => {
                assert_eq!(SSID, wifi.ssid);
                assert_eq!(PASSWORD, wifi.password);
                assert_eq!(WifiSecurity::WPA2, wifi.security);
            }
            _ => panic!("Should be a Wifi item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_wifi_minimal_data(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Open Network";
        const SSID: &str = "GuestWiFi";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        setup(&server); // Set up user data
        setup_paid_user(&server); // Override with paid plan
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let recorder = server.new_recorder();
        server.handler("/pass/v1/share/MyShareID/item", move |_| {
            success(CreateItemResponse {
                item: ItemRevision {
                    item_id: ITEM_ID.to_string(),
                    revision: 0,
                    content_format_version: 0,
                    key_rotation: 0,
                    content: "".to_string(),
                    item_key: None,
                    state: 0,
                    flags: 0,
                    alias_email: None,
                    create_time: 0,
                },
            })
        });

        let item_id = client
            .create_wifi(
                &share_id!(SHARE_ID),
                WifiItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    ssid: Some(SSID.to_string()),
                    password: None, // Open network
                    security: None,
                    note: None,
                },
            )
            .await
            .expect("Should be able to create the wifi item");

        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Decrypt and verify the item content
        let decoded_encrypted_item_key = crate::utils::b64_decode(&request.item_key).unwrap();
        let decrypted_item_key = pass_domain::crypto::decrypt(
            &decoded_encrypted_item_key,
            &TEST_SHARE_KEY,
            EncryptionTag::ItemKey,
        )
        .expect("Should be able to decrypt item key");

        let decoded_item_content = crate::utils::b64_decode(&request.content).unwrap();
        let decrypted_item_content = pass_domain::crypto::decrypt(
            &decoded_item_content,
            &decrypted_item_key,
            EncryptionTag::ItemContent,
        )
        .expect("Should be able to decrypt item content");

        let parsed_item_content = ItemData::deserialize(&decrypted_item_content)
            .expect("Should be able to deserialize ItemData");

        assert_eq!(ITEM_TITLE, parsed_item_content.title);

        match parsed_item_content.content {
            ItemContent::Wifi(wifi) => {
                assert_eq!(SSID, wifi.ssid);
                assert_eq!("", wifi.password); // Empty for open network
                assert_eq!(WifiSecurity::UnspecifiedWifiSecurity, wifi.security);
            }
            _ => panic!("Should be a Wifi item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_wifi_different_security_types(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Test Network";
        const SSID: &str = "TestSSID";
        const PASSWORD: &str = "password";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        setup(&server);
        setup_paid_user(&server);
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let recorder = server.new_recorder();
        server.handler("/pass/v1/share/MyShareID/item", move |_| {
            success(CreateItemResponse {
                item: ItemRevision {
                    item_id: ITEM_ID.to_string(),
                    revision: 0,
                    content_format_version: 0,
                    key_rotation: 0,
                    content: "".to_string(),
                    item_key: None,
                    state: 0,
                    flags: 0,
                    alias_email: None,
                    create_time: 0,
                },
            })
        });

        let _ = client
            .create_wifi(
                &share_id!(SHARE_ID),
                WifiItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    ssid: Some(SSID.to_string()),
                    password: Some(PASSWORD.to_string()),
                    security: Some(WifiSecurity::WPA3),
                    note: None,
                },
            )
            .await
            .expect("Should be able to create wifi with WPA3");

        let request: CreateItemRequest = last_request!(recorder);
        let decoded_encrypted_item_key = crate::utils::b64_decode(&request.item_key).unwrap();
        let decrypted_item_key = pass_domain::crypto::decrypt(
            &decoded_encrypted_item_key,
            &TEST_SHARE_KEY,
            EncryptionTag::ItemKey,
        )
        .unwrap();

        let decoded_item_content = crate::utils::b64_decode(&request.content).unwrap();
        let decrypted_item_content = pass_domain::crypto::decrypt(
            &decoded_item_content,
            &decrypted_item_key,
            EncryptionTag::ItemContent,
        )
        .unwrap();

        let parsed_item_content = ItemData::deserialize(&decrypted_item_content).unwrap();

        match parsed_item_content.content {
            ItemContent::Wifi(wifi) => {
                assert_eq!(WifiSecurity::WPA3, wifi.security);
            }
            _ => panic!("Should be a Wifi item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_wifi_invalid_ssid_empty(server: Arc<Server>) {
        const SHARE_ID: &str = "MyShareID";

        setup(&server);
        setup_paid_user(&server);
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let result = client
            .create_wifi(
                &share_id!(SHARE_ID),
                WifiItemCreatePayload {
                    title: "Test".to_string(),
                    ssid: Some("".to_string()),
                    password: None,
                    security: None,
                    note: None,
                },
            )
            .await;

        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("SSID") || err_string.contains("empty"),
            "Error should mention SSID or empty, got: {}",
            err_string
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_wifi_invalid_ssid_whitespace(server: Arc<Server>) {
        const SHARE_ID: &str = "MyShareID";

        setup(&server);
        setup_paid_user(&server);
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let result = client
            .create_wifi(
                &share_id!(SHARE_ID),
                WifiItemCreatePayload {
                    title: "Test".to_string(),
                    ssid: Some("   ".to_string()),
                    password: None,
                    security: None,
                    note: None,
                },
            )
            .await;

        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("SSID") || err_string.contains("whitespace"),
            "Error should mention SSID or whitespace, got: {}",
            err_string
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_wifi_unicode_ssid(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Unicode Network";
        const SSID: &str = "网络-测试";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        setup(&server);
        setup_paid_user(&server);
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let recorder = server.new_recorder();
        server.handler("/pass/v1/share/MyShareID/item", move |_| {
            success(CreateItemResponse {
                item: ItemRevision {
                    item_id: ITEM_ID.to_string(),
                    revision: 0,
                    content_format_version: 0,
                    key_rotation: 0,
                    content: "".to_string(),
                    item_key: None,
                    state: 0,
                    flags: 0,
                    alias_email: None,
                    create_time: 0,
                },
            })
        });

        let item_id = client
            .create_wifi(
                &share_id!(SHARE_ID),
                WifiItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    ssid: Some(SSID.to_string()),
                    password: Some("password".to_string()),
                    security: Some(WifiSecurity::WPA2),
                    note: None,
                },
            )
            .await
            .expect("Should be able to create wifi with unicode SSID");

        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);
        let decoded_encrypted_item_key = crate::utils::b64_decode(&request.item_key).unwrap();
        let decrypted_item_key = pass_domain::crypto::decrypt(
            &decoded_encrypted_item_key,
            &TEST_SHARE_KEY,
            EncryptionTag::ItemKey,
        )
        .unwrap();

        let decoded_item_content = crate::utils::b64_decode(&request.content).unwrap();
        let decrypted_item_content = pass_domain::crypto::decrypt(
            &decoded_item_content,
            &decrypted_item_key,
            EncryptionTag::ItemContent,
        )
        .unwrap();

        let parsed_item_content = ItemData::deserialize(&decrypted_item_content).unwrap();

        match parsed_item_content.content {
            ItemContent::Wifi(wifi) => {
                assert_eq!(SSID, wifi.ssid);
            }
            _ => panic!("Should be a Wifi item"),
        }
    }
}
