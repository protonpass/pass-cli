use crate::PassClient;
use crate::permission::PermissionAction;
use anyhow::{Context, Result};
use pass_domain::{IdentityItem, ItemContent, ItemId, ShareId};

#[derive(Clone, Debug)]
pub struct IdentityItemCreatePayload {
    pub title: String,
    pub note: Option<String>,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub phone_number: Option<String>,
    pub first_name: Option<String>,
    pub middle_name: Option<String>,
    pub last_name: Option<String>,
    pub birthdate: Option<String>,
    pub gender: Option<String>,
    pub organization: Option<String>,
    pub street_address: Option<String>,
    pub zip_or_postal_code: Option<String>,
    pub city: Option<String>,
    pub state_or_province: Option<String>,
    pub country_or_region: Option<String>,
    pub social_security_number: Option<String>,
    pub passport_number: Option<String>,
    pub license_number: Option<String>,
    pub website: Option<String>,
    pub company: Option<String>,
    pub job_title: Option<String>,
}

/// Trims a string value.
fn trim_value(value: &str) -> String {
    value.trim().to_string()
}

impl PassClient {
    pub async fn create_identity(
        &self,
        share_id: &ShareId,
        payload: IdentityItemCreatePayload,
    ) -> Result<ItemId> {
        // Check if user can create Identity
        self.action_guard(PermissionAction::CreateIdentity {
            share_id: share_id.clone(),
        })
        .await?;
        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                payload.note.as_deref().unwrap_or(""),
                ItemContent::Identity(Box::new(IdentityItem {
                    full_name: trim_value(&payload.full_name.unwrap_or_default()),
                    email: trim_value(&payload.email.unwrap_or_default()),
                    phone_number: trim_value(&payload.phone_number.unwrap_or_default()),
                    first_name: trim_value(&payload.first_name.unwrap_or_default()),
                    middle_name: trim_value(&payload.middle_name.unwrap_or_default()),
                    last_name: trim_value(&payload.last_name.unwrap_or_default()),
                    birthdate: trim_value(&payload.birthdate.unwrap_or_default()),
                    gender: trim_value(&payload.gender.unwrap_or_default()),
                    organization: trim_value(&payload.organization.unwrap_or_default()),
                    street_address: trim_value(&payload.street_address.unwrap_or_default()),
                    zip_or_postal_code: trim_value(&payload.zip_or_postal_code.unwrap_or_default()),
                    city: trim_value(&payload.city.unwrap_or_default()),
                    state_or_province: trim_value(&payload.state_or_province.unwrap_or_default()),
                    country_or_region: trim_value(&payload.country_or_region.unwrap_or_default()),
                    social_security_number: trim_value(
                        &payload.social_security_number.unwrap_or_default(),
                    ),
                    passport_number: trim_value(&payload.passport_number.unwrap_or_default()),
                    license_number: trim_value(&payload.license_number.unwrap_or_default()),
                    website: trim_value(&payload.website.unwrap_or_default()),
                    company: trim_value(&payload.company.unwrap_or_default()),
                    job_title: trim_value(&payload.job_title.unwrap_or_default()),
                })),
            )
            .await
            .context("Error creating identity item request")?;

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

    #[test]
    fn test_trim_value() {
        assert_eq!(trim_value("  value  "), "value");
        assert_eq!(trim_value("value"), "value");
        assert_eq!(trim_value("  "), "");
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_identity_full_data(server: Arc<Server>) {
        const ITEM_TITLE: &str = "John Doe";
        const ITEM_NOTE: &str = "Personal identity";
        const FULL_NAME: &str = "John Michael Doe";
        const EMAIL: &str = "john.doe@example.com";
        const PHONE: &str = "+1234567890";
        const FIRST_NAME: &str = "John";
        const MIDDLE_NAME: &str = "Michael";
        const LAST_NAME: &str = "Doe";
        const BIRTHDATE: &str = "1990-01-01";
        const GENDER: &str = "Male";
        const ORG: &str = "Acme Corp";
        const STREET: &str = "123 Main St";
        const ZIP: &str = "12345";
        const CITY: &str = "Springfield";
        const STATE: &str = "IL";
        const COUNTRY: &str = "USA";
        const SSN: &str = "123-45-6789";
        const PASSPORT: &str = "AB1234567";
        const LICENSE: &str = "D1234567";
        const WEBSITE: &str = "https://johndoe.com";
        const COMPANY: &str = "Tech Inc";
        const JOB_TITLE: &str = "Software Engineer";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client_with_plan(PlanType::Plus).await;
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
            .create_identity(
                &share_id!(SHARE_ID),
                IdentityItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: Some(ITEM_NOTE.to_string()),
                    full_name: Some(FULL_NAME.to_string()),
                    email: Some(EMAIL.to_string()),
                    phone_number: Some(PHONE.to_string()),
                    first_name: Some(FIRST_NAME.to_string()),
                    middle_name: Some(MIDDLE_NAME.to_string()),
                    last_name: Some(LAST_NAME.to_string()),
                    birthdate: Some(BIRTHDATE.to_string()),
                    gender: Some(GENDER.to_string()),
                    organization: Some(ORG.to_string()),
                    street_address: Some(STREET.to_string()),
                    zip_or_postal_code: Some(ZIP.to_string()),
                    city: Some(CITY.to_string()),
                    state_or_province: Some(STATE.to_string()),
                    country_or_region: Some(COUNTRY.to_string()),
                    social_security_number: Some(SSN.to_string()),
                    passport_number: Some(PASSPORT.to_string()),
                    license_number: Some(LICENSE.to_string()),
                    website: Some(WEBSITE.to_string()),
                    company: Some(COMPANY.to_string()),
                    job_title: Some(JOB_TITLE.to_string()),
                },
            )
            .await
            .expect("Should be able to create the identity item");

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
        assert_eq!(ITEM_NOTE, parsed_item_content.note);

        match parsed_item_content.content {
            ItemContent::Identity(identity) => {
                assert_eq!(FULL_NAME, identity.full_name);
                assert_eq!(EMAIL, identity.email);
                assert_eq!(PHONE, identity.phone_number);
                assert_eq!(FIRST_NAME, identity.first_name);
                assert_eq!(MIDDLE_NAME, identity.middle_name);
                assert_eq!(LAST_NAME, identity.last_name);
                assert_eq!(BIRTHDATE, identity.birthdate);
                assert_eq!(GENDER, identity.gender);
                assert_eq!(ORG, identity.organization);
                assert_eq!(STREET, identity.street_address);
                assert_eq!(ZIP, identity.zip_or_postal_code);
                assert_eq!(CITY, identity.city);
                assert_eq!(STATE, identity.state_or_province);
                assert_eq!(COUNTRY, identity.country_or_region);
                assert_eq!(SSN, identity.social_security_number);
                assert_eq!(PASSPORT, identity.passport_number);
                assert_eq!(LICENSE, identity.license_number);
                assert_eq!(WEBSITE, identity.website);
                assert_eq!(COMPANY, identity.company);
                assert_eq!(JOB_TITLE, identity.job_title);
            }
            _ => panic!("Should be an Identity item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_identity_minimal_data(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Minimal Identity";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client_with_plan(PlanType::Plus).await;
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
            .create_identity(
                &share_id!(SHARE_ID),
                IdentityItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: None,
                    full_name: None,
                    email: None,
                    phone_number: None,
                    first_name: None,
                    middle_name: None,
                    last_name: None,
                    birthdate: None,
                    gender: None,
                    organization: None,
                    street_address: None,
                    zip_or_postal_code: None,
                    city: None,
                    state_or_province: None,
                    country_or_region: None,
                    social_security_number: None,
                    passport_number: None,
                    license_number: None,
                    website: None,
                    company: None,
                    job_title: None,
                },
            )
            .await
            .expect("Should be able to create identity with minimal data");

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

        assert_eq!(ITEM_TITLE, parsed_item_content.title);

        match parsed_item_content.content {
            ItemContent::Identity(identity) => {
                assert_eq!("", identity.full_name);
                assert_eq!("", identity.email);
                assert_eq!("", identity.phone_number);
            }
            _ => panic!("Should be an Identity item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_identity_trimming(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Trimming Test";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client_with_plan(PlanType::Plus).await;
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
            .create_identity(
                &share_id!(SHARE_ID),
                IdentityItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: None,
                    full_name: Some("  John Doe  ".to_string()),
                    email: Some("  john@example.com  ".to_string()),
                    phone_number: Some("  +123456  ".to_string()),
                    first_name: None,
                    middle_name: None,
                    last_name: None,
                    birthdate: None,
                    gender: None,
                    organization: None,
                    street_address: None,
                    zip_or_postal_code: None,
                    city: None,
                    state_or_province: None,
                    country_or_region: None,
                    social_security_number: None,
                    passport_number: None,
                    license_number: None,
                    website: None,
                    company: None,
                    job_title: None,
                },
            )
            .await
            .expect("Should trim all values");

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
            ItemContent::Identity(identity) => {
                assert_eq!("John Doe", identity.full_name);
                assert_eq!("john@example.com", identity.email);
                assert_eq!("+123456", identity.phone_number);
            }
            _ => panic!("Should be an Identity item"),
        }
    }
}
