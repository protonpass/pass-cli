use crate::PassClient;
use crate::permission::PermissionAction;
use anyhow::{Context, Result, bail};
use pass_domain::{
    CustomItem, CustomSection, ItemContent, ItemExtraField, ItemExtraFieldContent, ItemId, ShareId,
};

#[derive(Clone, Debug)]
pub struct CustomItemCreatePayload {
    pub title: String,
    pub note: Option<String>,
    pub sections: Vec<CustomSectionPayload>,
}

#[derive(Clone, Debug)]
pub struct CustomSectionPayload {
    pub section_name: String,
    pub section_fields: Vec<CustomFieldPayload>,
}

#[derive(Clone, Debug)]
pub struct CustomFieldPayload {
    pub field_name: String,
    pub content: CustomFieldContentPayload,
}

#[derive(Clone, Debug)]
pub enum CustomFieldContentPayload {
    Text(String),
    Hidden(String),
    Totp(String),
    Timestamp(i64),
}

/// Validates a section name.
/// Section names must not be empty or whitespace-only after trimming.
fn validate_section_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("Section name cannot be empty or whitespace-only");
    }
    Ok(trimmed.to_string())
}

/// Validates a field name.
/// Field names must not be empty or whitespace-only after trimming.
fn validate_field_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("Field name cannot be empty or whitespace-only");
    }
    Ok(trimmed.to_string())
}

/// Trims and returns a string value.
fn trim_value(value: &str) -> String {
    value.trim().to_string()
}

impl CustomFieldPayload {
    /// Converts payload to domain model with validation and trimming.
    fn into_extra_field(self) -> Result<ItemExtraField> {
        let field_name = validate_field_name(&self.field_name)?;

        let content = match self.content {
            CustomFieldContentPayload::Text(value) => {
                ItemExtraFieldContent::Text(trim_value(&value))
            }
            CustomFieldContentPayload::Hidden(value) => {
                ItemExtraFieldContent::Hidden(trim_value(&value))
            }
            CustomFieldContentPayload::Totp(value) => {
                ItemExtraFieldContent::Totp(trim_value(&value))
            }
            CustomFieldContentPayload::Timestamp(ts) => ItemExtraFieldContent::Timestamp(ts),
        };

        Ok(ItemExtraField {
            name: field_name,
            content,
        })
    }
}

impl CustomSectionPayload {
    /// Converts payload to domain model with validation and trimming.
    fn into_custom_section(self) -> Result<CustomSection> {
        let section_name = validate_section_name(&self.section_name)?;

        if self.section_fields.is_empty() {
            warn!("Section '{}' has no fields", section_name);
        }

        let section_fields: Result<Vec<ItemExtraField>> = self
            .section_fields
            .into_iter()
            .map(|f| f.into_extra_field())
            .collect();

        Ok(CustomSection {
            section_name,
            section_fields: section_fields?,
        })
    }
}

impl PassClient {
    pub async fn create_custom(
        &self,
        share_id: &ShareId,
        payload: CustomItemCreatePayload,
    ) -> Result<ItemId> {
        self.action_guard(PermissionAction::CreateCustomItem {
            share_id: share_id.clone(),
        })
        .await?;
        // Validate and convert sections
        let sections: Result<Vec<CustomSection>> = payload
            .sections
            .into_iter()
            .map(|s| s.into_custom_section())
            .collect();
        let sections = sections.context("Error validating custom item sections")?;

        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                payload.note.as_deref().unwrap_or(""),
                ItemContent::Custom(CustomItem { sections }),
            )
            .await
            .context("Error creating custom item request")?;

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

    // Unit tests for validation functions
    #[test]
    fn test_validate_section_name_valid() {
        assert_eq!(validate_section_name("Section 1").unwrap(), "Section 1");
        assert_eq!(validate_section_name("  Section 1  ").unwrap(), "Section 1");
        assert_eq!(
            validate_section_name("My-Section_123").unwrap(),
            "My-Section_123"
        );
    }

    #[test]
    fn test_validate_section_name_invalid() {
        assert!(validate_section_name("").is_err());
        assert!(validate_section_name("   ").is_err());
        assert!(validate_section_name("\t\n").is_err());
    }

    #[test]
    fn test_validate_field_name_valid() {
        assert_eq!(validate_field_name("Field 1").unwrap(), "Field 1");
        assert_eq!(validate_field_name("  Field 1  ").unwrap(), "Field 1");
        assert_eq!(validate_field_name("My-Field_123").unwrap(), "My-Field_123");
    }

    #[test]
    fn test_validate_field_name_invalid() {
        assert!(validate_field_name("").is_err());
        assert!(validate_field_name("   ").is_err());
        assert!(validate_field_name("\t\n").is_err());
    }

    #[test]
    fn test_trim_value() {
        assert_eq!(trim_value("  value  "), "value");
        assert_eq!(trim_value("value"), "value");
        assert_eq!(trim_value("  "), "");
    }

    // Integration tests using muon test server
    #[muon::test(scheme(HTTP))]
    async fn test_create_custom_with_all_field_types(server: Arc<Server>) {
        const ITEM_TITLE: &str = "My Custom Item";
        const ITEM_NOTE: &str = "Custom note";
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
            .create_custom(
                &share_id!(SHARE_ID),
                CustomItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: Some(ITEM_NOTE.to_string()),
                    sections: vec![CustomSectionPayload {
                        section_name: "Section 1".to_string(),
                        section_fields: vec![
                            CustomFieldPayload {
                                field_name: "Text Field".to_string(),
                                content: CustomFieldContentPayload::Text("text value".to_string()),
                            },
                            CustomFieldPayload {
                                field_name: "Hidden Field".to_string(),
                                content: CustomFieldContentPayload::Hidden("secret".to_string()),
                            },
                            CustomFieldPayload {
                                field_name: "TOTP Field".to_string(),
                                content: CustomFieldContentPayload::Totp(
                                    "otpauth://totp/test".to_string(),
                                ),
                            },
                            CustomFieldPayload {
                                field_name: "Timestamp Field".to_string(),
                                content: CustomFieldContentPayload::Timestamp(1730000000),
                            },
                        ],
                    }],
                },
            )
            .await
            .expect("Should be able to create the custom item");

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
            ItemContent::Custom(custom) => {
                assert_eq!(1, custom.sections.len());
                let section = &custom.sections[0];
                assert_eq!("Section 1", section.section_name);
                assert_eq!(4, section.section_fields.len());

                // Verify field types
                assert!(matches!(
                    &section.section_fields[0].content,
                    ItemExtraFieldContent::Text(_)
                ));
                assert!(matches!(
                    &section.section_fields[1].content,
                    ItemExtraFieldContent::Hidden(_)
                ));
                assert!(matches!(
                    &section.section_fields[2].content,
                    ItemExtraFieldContent::Totp(_)
                ));
                assert!(matches!(
                    &section.section_fields[3].content,
                    ItemExtraFieldContent::Timestamp(_)
                ));
            }
            _ => panic!("Should be a Custom item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_custom_empty_sections(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Empty Custom Item";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client_with_plan(PlanType::Plus).await;
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

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
            .create_custom(
                &share_id!(SHARE_ID),
                CustomItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: None,
                    sections: vec![],
                },
            )
            .await
            .expect("Should be able to create custom item with empty sections");

        assert_eq!(ITEM_ID, item_id.value());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_custom_trimming(server: Arc<Server>) {
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
            .create_custom(
                &share_id!(SHARE_ID),
                CustomItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: None,
                    sections: vec![CustomSectionPayload {
                        section_name: "  Section Name  ".to_string(),
                        section_fields: vec![CustomFieldPayload {
                            field_name: "  Field Name  ".to_string(),
                            content: CustomFieldContentPayload::Text("  value  ".to_string()),
                        }],
                    }],
                },
            )
            .await
            .expect("Should trim names and values");

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
            ItemContent::Custom(custom) => {
                assert_eq!("Section Name", custom.sections[0].section_name);
                assert_eq!("Field Name", custom.sections[0].section_fields[0].name);
                if let ItemExtraFieldContent::Text(value) =
                    &custom.sections[0].section_fields[0].content
                {
                    assert_eq!("value", value);
                } else {
                    panic!("Expected Text field");
                }
            }
            _ => panic!("Should be a Custom item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_custom_invalid_empty_section_name(server: Arc<Server>) {
        const SHARE_ID: &str = "MyShareID";

        let client = server.pass_client_with_plan(PlanType::Plus).await;
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let result = client
            .create_custom(
                &share_id!(SHARE_ID),
                CustomItemCreatePayload {
                    title: "Test".to_string(),
                    note: None,
                    sections: vec![CustomSectionPayload {
                        section_name: "   ".to_string(),
                        section_fields: vec![],
                    }],
                },
            )
            .await;

        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("Section name")
                || err_string.contains("empty")
                || err_string.contains("validating"),
            "Error should be related to validation, got: {}",
            err_string
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_custom_invalid_empty_field_name(server: Arc<Server>) {
        const SHARE_ID: &str = "MyShareID";

        let client = server.pass_client_with_plan(PlanType::Plus).await;
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let result = client
            .create_custom(
                &share_id!(SHARE_ID),
                CustomItemCreatePayload {
                    title: "Test".to_string(),
                    note: None,
                    sections: vec![CustomSectionPayload {
                        section_name: "Section".to_string(),
                        section_fields: vec![CustomFieldPayload {
                            field_name: "   ".to_string(),
                            content: CustomFieldContentPayload::Text("value".to_string()),
                        }],
                    }],
                },
            )
            .await;

        assert!(result.is_err());
        let err_string = result.unwrap_err().to_string();
        assert!(
            err_string.contains("Field name")
                || err_string.contains("empty")
                || err_string.contains("validating"),
            "Error should be related to validation, got: {}",
            err_string
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_custom_multiple_sections(server: Arc<Server>) {
        const ITEM_TITLE: &str = "Multi-Section Item";
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
            .create_custom(
                &share_id!(SHARE_ID),
                CustomItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: None,
                    sections: vec![
                        CustomSectionPayload {
                            section_name: "Section 1".to_string(),
                            section_fields: vec![CustomFieldPayload {
                                field_name: "Field 1".to_string(),
                                content: CustomFieldContentPayload::Text("value1".to_string()),
                            }],
                        },
                        CustomSectionPayload {
                            section_name: "Section 2".to_string(),
                            section_fields: vec![CustomFieldPayload {
                                field_name: "Field 2".to_string(),
                                content: CustomFieldContentPayload::Hidden("secret".to_string()),
                            }],
                        },
                    ],
                },
            )
            .await
            .expect("Should create item with multiple sections");

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
            ItemContent::Custom(custom) => {
                assert_eq!(2, custom.sections.len());
                assert_eq!("Section 1", custom.sections[0].section_name);
                assert_eq!("Section 2", custom.sections[1].section_name);
            }
            _ => panic!("Should be a Custom item"),
        }
    }
}
