use super::ItemCreatedEvent;
use crate::PassClient;
use anyhow::{Context, Result};
use pass_domain::{
    FolderId, ItemContent, ItemData, ItemExtraField, ItemExtraFieldContent, ItemId, ItemType,
    ShareId, SshKeyItem,
};

#[derive(Clone, Debug)]
pub struct SshKeyItemCreatePayload {
    pub title: String,
    pub private_key: String,
    pub public_key: String,
    pub passphrase: Option<String>,
}

impl PassClient {
    pub async fn create_ssh_key(
        &self,
        share_id: &ShareId,
        payload: SshKeyItemCreatePayload,
        folder_id: Option<&FolderId>,
    ) -> Result<ItemId> {
        let mut extra_fields = vec![];

        // If passphrase is provided, add it as a hidden field
        if let Some(passphrase) = payload.passphrase {
            extra_fields.push(ItemExtraField {
                name: "Passphrase".to_string(),
                content: ItemExtraFieldContent::Hidden(passphrase),
            });
        }

        let content = ItemData {
            title: payload.title.to_string(),
            note: String::new(),
            item_uuid: ItemData::generate_uuid(),
            content: ItemContent::SshKey(SshKeyItem {
                private_key: payload.private_key,
                public_key: payload.public_key,
            }),
            extra_fields,
        };

        let req = self
            .create_item_request_from_data(share_id, content, folder_id)
            .await
            .context("Error creating item request")?;

        let item_id = self
            .send_create_item_request(share_id, req)
            .await
            .context("Error sending create item request")?;

        self.emit_telemetry(&ItemCreatedEvent {
            item_type: ItemType::SshKey,
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

    #[muon::test(scheme(HTTP))]
    async fn test_create_ssh_key_with_passphrase(server: Arc<Server>) {
        const ITEM_TITLE: &str = "MySSHKey";
        const PRIVATE_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\ntest_private_key\n-----END OPENSSH PRIVATE KEY-----";
        const PUBLIC_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITest test@example.com";
        const PASSPHRASE: &str = "MySecurePassphrase123!";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client().await;
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
            .create_ssh_key(
                &share_id!(SHARE_ID),
                SshKeyItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    private_key: PRIVATE_KEY.to_string(),
                    public_key: PUBLIC_KEY.to_string(),
                    passphrase: Some(PASSPHRASE.to_string()),
                },
                None,
            )
            .await
            .expect("Should be able to create the SSH key item");

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
        assert_eq!("", parsed_item_content.note);

        let ssh_key_item = match parsed_item_content.content {
            ItemContent::SshKey(ssh_key_item) => ssh_key_item,
            _ => panic!("Should be an SshKey item"),
        };

        assert_eq!(PRIVATE_KEY, ssh_key_item.private_key);
        assert_eq!(PUBLIC_KEY, ssh_key_item.public_key);

        // Verify passphrase is stored as an extra hidden field
        assert_eq!(1, parsed_item_content.extra_fields.len());
        let passphrase_field = &parsed_item_content.extra_fields[0];
        assert_eq!("Passphrase", passphrase_field.name);
        match &passphrase_field.content {
            ItemExtraFieldContent::Hidden(value) => {
                assert_eq!(PASSPHRASE, value);
            }
            _ => panic!("Passphrase should be a hidden field"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_ssh_key_without_passphrase(server: Arc<Server>) {
        const ITEM_TITLE: &str = "MyUnprotectedSSHKey";
        const PRIVATE_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\ntest_private_key_no_pass\n-----END OPENSSH PRIVATE KEY-----";
        const PUBLIC_KEY: &str = "ssh-rsa AAAAB3NzaC1yc2EAAAATest test@example.com";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client().await;
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
            .create_ssh_key(
                &share_id!(SHARE_ID),
                SshKeyItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    private_key: PRIVATE_KEY.to_string(),
                    public_key: PUBLIC_KEY.to_string(),
                    passphrase: None,
                },
                None,
            )
            .await
            .expect("Should be able to create the SSH key item");

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
        assert_eq!("", parsed_item_content.note);

        let ssh_key_item = match parsed_item_content.content {
            ItemContent::SshKey(ssh_key_item) => ssh_key_item,
            _ => panic!("Should be an SshKey item"),
        };

        assert_eq!(PRIVATE_KEY, ssh_key_item.private_key);
        assert_eq!(PUBLIC_KEY, ssh_key_item.public_key);

        // Verify no extra fields when passphrase is not provided
        assert_eq!(0, parsed_item_content.extra_fields.len());
    }
}
