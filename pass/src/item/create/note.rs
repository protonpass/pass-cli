use super::ItemCreatedEvent;
use crate::PassClient;
use anyhow::{Context, Result};
use pass_domain::{FolderId, ItemContent, ItemId, ItemType, NoteItem, ShareId};

#[derive(Clone, Debug)]
pub struct NoteItemCreatePayload {
    pub title: String,
    pub note: Option<String>,
}

impl PassClient {
    pub async fn create_note(
        &self,
        share_id: &ShareId,
        payload: NoteItemCreatePayload,
        folder_id: Option<&FolderId>,
    ) -> Result<ItemId> {
        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                &payload.note.unwrap_or_default(),
                ItemContent::Note(NoteItem),
                folder_id,
            )
            .await
            .context("Error creating note item request")?;

        let item_id = self.send_create_item_request(share_id, req).await?;

        self.emit_telemetry(&ItemCreatedEvent {
            item_type: ItemType::Note,
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
    async fn test_create_note(server: Arc<Server>) {
        const ITEM_TITLE: &str = "MyNoteItem";
        const ITEM_NOTE: &str = "This is my note content";
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
            .create_note(
                &share_id!(SHARE_ID),
                NoteItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: Some(ITEM_NOTE.to_string()),
                },
                None,
            )
            .await
            .expect("Should be able to create the item");

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
        assert_eq!(ITEM_NOTE, parsed_item_content.note);

        match parsed_item_content.content {
            ItemContent::Note(_) => {
                // Success - we have a Note item
            }
            _ => panic!("Should be a Note item"),
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_note_without_note_content(server: Arc<Server>) {
        const ITEM_TITLE: &str = "MyNoteItem";
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
            .create_note(
                &share_id!(SHARE_ID),
                NoteItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    note: None,
                },
                None,
            )
            .await
            .expect("Should be able to create the item");

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

        match parsed_item_content.content {
            ItemContent::Note(_) => {
                // Success - we have a Note item
            }
            _ => panic!("Should be a Note item"),
        }
    }
}
