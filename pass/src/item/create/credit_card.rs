use super::ItemCreatedEvent;
use crate::PassClient;
use crate::permission::PermissionAction;
use anyhow::{Context, Result, bail};
use pass_domain::{CardType, CreditCardItem, FolderId, ItemContent, ItemId, ItemType, ShareId};

#[derive(Clone, Debug)]
pub struct CreditCardItemCreatePayload {
    pub title: String,
    pub cardholder_name: Option<String>,
    pub number: Option<String>,
    pub verification_number: Option<String>,
    pub expiration_date: Option<String>,
    pub pin: Option<String>,
    pub note: Option<String>,
}

/// Sanitizes a card number by removing all non-digit characters.
/// This removes spaces, hyphens, and any other non-numeric characters,
/// leaving only consecutive digits.
/// Empty string returns empty string.
fn sanitize_card_number(card_number: &str) -> String {
    card_number.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Validates expiration date format and value.
/// Format must be YYYY-MM (e.g., "2025-12")
/// Year must be 4 digits (2000-9999 range)
/// Month must be 01-12 (with leading zero)
/// Empty string is allowed (optional field)
fn validate_expiration_date(date: &str) -> Result<()> {
    // Empty string is valid (optional field)
    if date.is_empty() {
        return Ok(());
    }

    // Check for whitespace-only strings
    if date.trim().is_empty() {
        bail!("Expiration date cannot be whitespace only");
    }

    // Check format: YYYY-MM
    if date.len() != 7 {
        bail!(
            "Expiration date must be in format YYYY-MM (e.g., 2025-12), got: {}",
            date
        );
    }

    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 2 {
        bail!(
            "Expiration date must contain exactly one dash separator (YYYY-MM), got: {}",
            date
        );
    }

    // Validate year part
    let year_str = parts[0];
    if year_str.len() != 4 {
        bail!("Year must be 4 digits (YYYY), got: {}", year_str);
    }

    let year = year_str
        .parse::<u32>()
        .with_context(|| format!("Year must be a valid number, got: {}", year_str))?;

    // Basic sanity check for year range
    if !(2000..=9999).contains(&year) {
        bail!(
            "Year must be in reasonable range (2000-9999), got: {}",
            year
        );
    }

    // Validate month part
    let month_str = parts[1];
    if month_str.len() != 2 {
        bail!(
            "Month must be 2 digits with leading zero (01-12), got: {}",
            month_str
        );
    }

    let month = month_str
        .parse::<u32>()
        .with_context(|| format!("Month must be a valid number, got: {}", month_str))?;

    if !(1..=12).contains(&month) {
        bail!("Month must be between 01 and 12, got: {}", month_str);
    }

    Ok(())
}

impl PassClient {
    pub async fn create_credit_card(
        &self,
        share_id: &ShareId,
        payload: CreditCardItemCreatePayload,
        folder_id: Option<&FolderId>,
    ) -> Result<ItemId> {
        // Check if user has a paid plan
        self.action_guard(PermissionAction::CreateCreditCard {
            share_id: share_id.clone(),
        })
        .await?;

        // Validate expiration date if provided
        let expiration_date = payload.expiration_date.as_deref().unwrap_or("");
        validate_expiration_date(expiration_date).context("Invalid expiration date")?;

        // Sanitize card number to remove spaces, hyphens, and other non-digit characters
        let sanitized_number = payload
            .number
            .as_deref()
            .map(sanitize_card_number)
            .unwrap_or_default();

        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                payload.note.as_deref().unwrap_or(""),
                ItemContent::CreditCard(CreditCardItem {
                    cardholder_name: payload.cardholder_name.unwrap_or_default(),
                    card_type: CardType::Unspecified, // Unused for now
                    number: sanitized_number,
                    verification_number: payload.verification_number.unwrap_or_default(),
                    expiration_date: expiration_date.to_string(),
                    pin: payload.pin.unwrap_or_default(),
                }),
                folder_id,
            )
            .await
            .context("Error creating credit card item request")?;

        let item_id = self.send_create_item_request(share_id, req).await?;

        self.emit_telemetry(&ItemCreatedEvent {
            item_type: ItemType::CreditCard,
        })
        .await;

        Ok(item_id)
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

    // Unit tests for sanitize_card_number function
    #[test]
    fn test_sanitize_card_number_with_spaces() {
        assert_eq!(
            sanitize_card_number("4111 1111 1111 1111"),
            "4111111111111111"
        );
    }

    #[test]
    fn test_sanitize_card_number_with_hyphens() {
        assert_eq!(
            sanitize_card_number("4111-1111-1111-1111"),
            "4111111111111111"
        );
    }

    #[test]
    fn test_sanitize_card_number_with_mixed_separators() {
        assert_eq!(
            sanitize_card_number("4111 1111-1111 1111"),
            "4111111111111111"
        );
        assert_eq!(
            sanitize_card_number("4111_1111.1111-1111"),
            "4111111111111111"
        );
    }

    #[test]
    fn test_sanitize_card_number_already_clean() {
        assert_eq!(sanitize_card_number("4111111111111111"), "4111111111111111");
    }

    #[test]
    fn test_sanitize_card_number_with_special_characters() {
        assert_eq!(
            sanitize_card_number("4111*1111#1111@1111"),
            "4111111111111111"
        );
    }

    #[test]
    fn test_sanitize_card_number_with_letters() {
        assert_eq!(
            sanitize_card_number("4111abc1111def1111ghi1111"),
            "4111111111111111"
        );
    }

    #[test]
    fn test_sanitize_card_number_empty_string() {
        assert_eq!(sanitize_card_number(""), "");
    }

    #[test]
    fn test_sanitize_card_number_only_non_digits() {
        assert_eq!(sanitize_card_number("----    "), "");
        assert_eq!(sanitize_card_number("abcd efgh"), "");
    }

    #[test]
    fn test_sanitize_card_number_with_whitespace_variations() {
        assert_eq!(
            sanitize_card_number("4111\t1111\n1111\r1111"),
            "4111111111111111"
        );
    }

    // Unit tests for validate_expiration_date function
    #[test]
    fn test_validate_expiration_date_valid() {
        // Valid formats
        assert!(validate_expiration_date("2025-12").is_ok());
        assert!(validate_expiration_date("2030-01").is_ok());
        assert!(validate_expiration_date("2025-06").is_ok());
        assert!(validate_expiration_date("2099-11").is_ok());
        assert!(validate_expiration_date("2000-01").is_ok());
        assert!(validate_expiration_date("9999-12").is_ok());

        // Empty string is valid (optional field)
        assert!(validate_expiration_date("").is_ok());
    }

    #[test]
    fn test_validate_expiration_date_invalid_format() {
        // Wrong year format
        assert!(validate_expiration_date("25-12").is_err());
        assert!(validate_expiration_date("202-12").is_err());
        assert!(validate_expiration_date("20255-12").is_err());

        // Wrong separator
        assert!(validate_expiration_date("2025/12").is_err());
        assert!(validate_expiration_date("2025.12").is_err());
        assert!(validate_expiration_date("202512").is_err());

        // Missing leading zero in month
        assert!(validate_expiration_date("2025-1").is_err());
        assert!(validate_expiration_date("2025-9").is_err());

        // Whitespace only
        assert!(validate_expiration_date("   ").is_err());
        assert!(validate_expiration_date("\t").is_err());
    }

    #[test]
    fn test_validate_expiration_date_invalid_month() {
        assert!(validate_expiration_date("2025-00").is_err());
        assert!(validate_expiration_date("2025-13").is_err());
        assert!(validate_expiration_date("2025-99").is_err());
        assert!(validate_expiration_date("2030-20").is_err());
    }

    #[test]
    fn test_validate_expiration_date_invalid_year() {
        // Years outside reasonable range should fail
        assert!(validate_expiration_date("1999-12").is_err());
        assert!(validate_expiration_date("1900-06").is_err());
        assert!(validate_expiration_date("0000-01").is_err());
    }

    #[test]
    fn test_validate_expiration_date_invalid_values() {
        // Non-numeric values
        assert!(validate_expiration_date("ABCD-12").is_err());
        assert!(validate_expiration_date("2025-AB").is_err());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_credit_card_full_data(server: Arc<Server>) {
        const ITEM_TITLE: &str = "My Visa Card";
        const CARDHOLDER_NAME: &str = "John Doe";
        const CARD_NUMBER: &str = "4111111111111111";
        const CVV: &str = "123";
        const EXPIRATION_DATE: &str = "2027-12";
        const PIN: &str = "1234";
        const NOTE: &str = "Primary card";
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

        let handled = server.handler_with_method(
            Method::POST,
            format!("/pass/v1/share/{SHARE_ID}/item"),
            move |_| {
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
                        folder_id: None,
                    },
                })
            },
        );

        let recorder = server.new_recorder();
        let item_id = client
            .create_credit_card(
                &share_id!(SHARE_ID),
                CreditCardItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    cardholder_name: Some(CARDHOLDER_NAME.to_string()),
                    number: Some(CARD_NUMBER.to_string()),
                    verification_number: Some(CVV.to_string()),
                    expiration_date: Some(EXPIRATION_DATE.to_string()),
                    pin: Some(PIN.to_string()),
                    note: Some(NOTE.to_string()),
                },
                None,
            )
            .await
            .expect("Should be able to create the credit card item");

        assert_hit!(handled);
        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Check item is properly encrypted and contains the right contents
        let decoded_encrypted_item_key = crate::utils::b64_decode(&request.item_key).unwrap();
        let decrypted_item_key = pass_domain::crypto::decrypt(
            &decoded_encrypted_item_key,
            &TEST_SHARE_KEY,
            EncryptionTag::ItemKey,
        )
        .expect("Should be able to decrypt item key");
        assert_eq!(32, decrypted_item_key.len());

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

        let credit_card = match parsed_item_content.content {
            ItemContent::CreditCard(cc) => cc,
            _ => panic!("Should be a CreditCard item"),
        };

        assert_eq!(CARDHOLDER_NAME, credit_card.cardholder_name);
        assert_eq!(CardType::Unspecified, credit_card.card_type);
        assert_eq!(CARD_NUMBER, credit_card.number);
        assert_eq!(CVV, credit_card.verification_number);
        assert_eq!(EXPIRATION_DATE, credit_card.expiration_date);
        assert_eq!(PIN, credit_card.pin);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_credit_card_minimal_data(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Minimal Card";
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

        let handled = server.handler_with_method(
            Method::POST,
            format!("/pass/v1/share/{SHARE_ID}/item"),
            move |_| {
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
                        folder_id: None,
                    },
                })
            },
        );

        let recorder = server.new_recorder();
        let item_id = client
            .create_credit_card(
                &share_id!(SHARE_ID),
                CreditCardItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    cardholder_name: None,
                    number: None,
                    verification_number: None,
                    expiration_date: None,
                    pin: None,
                    note: None,
                },
                None,
            )
            .await
            .expect("Should be able to create the credit card item with minimal data");

        assert_hit!(handled);
        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Check item is properly encrypted
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
        assert_eq!("", parsed_item_content.note);

        match parsed_item_content.content {
            ItemContent::CreditCard(cc) => {
                assert_eq!("", cc.cardholder_name);
                assert_eq!(CardType::Unspecified, cc.card_type);
                assert_eq!("", cc.number);
                assert_eq!("", cc.verification_number);
                assert_eq!("", cc.expiration_date);
                assert_eq!("", cc.pin);
            }
            _ => panic!("Should be a CreditCard item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_credit_card_invalid_expiration_date(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Invalid Date Card";
        const SHARE_ID: &str = "MyShareID";

        setup(&server); // Set up user data
        setup_paid_user(&server); // Override with paid plan
        let client = server.pass_client_no_setup().await;
        client
            .setup_key_passphrases(TEST_PASSPHRASE)
            .await
            .expect("Error setting up passphrases");
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        // Test with invalid format (wrong separator)
        let result = client
            .create_credit_card(
                &share_id!(SHARE_ID),
                CreditCardItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    cardholder_name: None,
                    number: None,
                    verification_number: None,
                    expiration_date: Some("2025/12".to_string()), // Wrong separator
                    pin: None,
                    note: None,
                },
                None,
            )
            .await;

        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("expiration date") || err_string.contains("YYYY-MM"),
            "Error should mention expiration date format, got: {}",
            err_string
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_credit_card_sanitizes_card_number(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Card with Formatted Number";
        const CARD_NUMBER_WITH_SPACES: &str = "4111 1111 1111 1111";
        const CARD_NUMBER_SANITIZED: &str = "4111111111111111";
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

        let handled = server.handler_with_method(
            Method::POST,
            format!("/pass/v1/share/{SHARE_ID}/item"),
            move |_| {
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
                        folder_id: None,
                    },
                })
            },
        );

        let recorder = server.new_recorder();
        let item_id = client
            .create_credit_card(
                &share_id!(SHARE_ID),
                CreditCardItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    cardholder_name: None,
                    number: Some(CARD_NUMBER_WITH_SPACES.to_string()),
                    verification_number: None,
                    expiration_date: None,
                    pin: None,
                    note: None,
                },
                None,
            )
            .await
            .expect("Should be able to create the credit card item");

        assert_hit!(handled);
        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Check that the card number was sanitized (spaces removed)
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

        match parsed_item_content.content {
            ItemContent::CreditCard(cc) => {
                assert_eq!(
                    CARD_NUMBER_SANITIZED, cc.number,
                    "Card number should be sanitized (spaces removed)"
                );
            }
            _ => panic!("Should be a CreditCard item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_credit_card_sanitizes_card_number_with_hyphens(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Card with Hyphens";
        const CARD_NUMBER_WITH_HYPHENS: &str = "4111-1111-1111-1111";
        const CARD_NUMBER_SANITIZED: &str = "4111111111111111";
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

        let handled = server.handler_with_method(
            Method::POST,
            format!("/pass/v1/share/{SHARE_ID}/item"),
            move |_| {
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
                        folder_id: None,
                    },
                })
            },
        );

        let recorder = server.new_recorder();
        let item_id = client
            .create_credit_card(
                &share_id!(SHARE_ID),
                CreditCardItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    cardholder_name: None,
                    number: Some(CARD_NUMBER_WITH_HYPHENS.to_string()),
                    verification_number: None,
                    expiration_date: None,
                    pin: None,
                    note: None,
                },
                None,
            )
            .await
            .expect("Should be able to create the credit card item");

        assert_hit!(handled);
        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Check that the card number was sanitized (hyphens removed)
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

        match parsed_item_content.content {
            ItemContent::CreditCard(cc) => {
                assert_eq!(
                    CARD_NUMBER_SANITIZED, cc.number,
                    "Card number should be sanitized (hyphens removed)"
                );
            }
            _ => panic!("Should be a CreditCard item"),
        }
    }
}
