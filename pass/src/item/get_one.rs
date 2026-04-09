use crate::item::item_keys::OpenedItemKey;
use crate::item::list::ItemRevision;
use crate::pagination::SincePagination;
use crate::utils::debug_response;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{
    AttachmentChunk, AttachmentId, Item, ItemAttachment, ItemAttachmentContent, ItemFlag, ItemId,
    ShareId, crypto,
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct ItemDetails {
    pub item: Item,
    pub attachments: Vec<ItemAttachment>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct GetItemResponse {
    #[serde(rename = "Item")]
    pub item: ItemRevision,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct GetAttachmentsResponse {
    #[serde(rename = "Files")]
    pub files: AttachmentsResponse,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct AttachmentsResponse {
    #[serde(rename = "Files")]
    pub files: Vec<AttachmentResponse>,
    #[serde(rename = "LastID")]
    pub last_id: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct AttachmentResponse {
    #[serde(rename = "FileID")]
    pub file_id: String,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "Metadata")]
    pub metadata: String,
    #[serde(rename = "FileKey")]
    pub file_key: String,
    #[serde(rename = "Chunks")]
    pub chunks: Vec<ChunkResponse>,
    #[serde(rename = "EncryptionVersion")]
    pub encryption_version: u8,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct ChunkResponse {
    #[serde(rename = "ChunkID")]
    pub chunk_id: String,
    #[serde(rename = "Index")]
    pub index: usize,
    #[serde(rename = "Size")]
    pub size: usize,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn view_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<ItemDetails> {
        let item_revision = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item")?;

        let opened = self
            .open_items(share_id, vec![item_revision])
            .await
            .context("Error opening item")?;
        let (item, item_key) = match opened.first().cloned() {
            Some(item) => (item.item, item.item_key),
            None => return Err(anyhow!("Item not found")),
        };

        let attachments = self
            .retrieve_attachments(&item, item_key)
            .await
            .context("Error fetching attachments")?;

        Ok(ItemDetails { item, attachments })
    }

    async fn retrieve_attachments(
        &self,
        item: &Item,
        item_key: OpenedItemKey,
    ) -> Result<Vec<ItemAttachment>> {
        if !item.flags.contains(&ItemFlag::ItemHasFiles) {
            debug!("Item does not have files, not fetching attachments");
            return Ok(Vec::new());
        }

        let attachments = self
            .fetch_attachments(&item.share_id, &item.id)
            .await
            .context("Error fetching attachments")?;

        let mut res = Vec::with_capacity(attachments.len());

        for attachment in attachments {
            let decoded_attachment_key = crate::utils::b64_decode(&attachment.file_key)
                .context("Error decoding attachment key")?;
            let decrypted_attachment_key = crypto::decrypt(
                &decoded_attachment_key,
                item_key.key.as_ref(),
                crypto::EncryptionTag::FileKey,
            )
            .map_err(|e| {
                error!("Error decrypting file key: {e:#}");
                anyhow!("Error decrypting file key")
            })?;

            let decoded_metadata = crate::utils::b64_decode(&attachment.metadata)
                .context("Error decoding attachment metadata")?;
            let decrypted_metadata = match attachment.encryption_version {
                1 => crypto::decrypt(
                    &decoded_metadata,
                    &decrypted_attachment_key,
                    crypto::EncryptionTag::FileData,
                ),
                2 => crypto::decrypt(
                    &decoded_metadata,
                    &decrypted_attachment_key,
                    crypto::EncryptionTag::FileMetadata,
                ),
                _ => {
                    return Err(anyhow!(
                        "Unsupported file encryption version {}",
                        attachment.encryption_version
                    ));
                }
            }
            .map_err(|e| {
                error!("Error decrypting file metadata: {e:#}");
                anyhow!("Error decrypting file metadata")
            })?;

            let content = ItemAttachmentContent::deserialize(&decrypted_metadata)
                .context("Error parsing attachment metadata")?;

            let chunks = attachment
                .chunks
                .into_iter()
                .map(|chunk| AttachmentChunk {
                    chunk_id: chunk.chunk_id,
                    index: chunk.index,
                    size: chunk.size,
                })
                .collect();

            res.push(ItemAttachment {
                id: AttachmentId::new(attachment.file_id),
                size: attachment.size,
                encryption_version: attachment.encryption_version,
                encrypted_file_key: crate::utils::b64_decode(&attachment.file_key)
                    .context("Error decoding attachment key")?,
                content,
                chunks,
            })
        }

        Ok(res)
    }

    async fn fetch_attachments(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<Vec<AttachmentResponse>> {
        let mut attachments = Vec::new();
        let mut pagination = SincePagination::default();
        loop {
            let mut req = GET!(
                "/pass/v1/share/{share_id}/item/{item_id}/files",
                share_id = share_id.value(),
                item_id = item_id.value()
            )
            .query(("PageSize".to_string(), format!("{}", pagination.page_size)));

            if let Some(ref since) = pagination.since {
                req = req.query(("Since".to_string(), since.to_string()));
            }

            let res = self.send(req).await.context("Error fetching items page")?;

            if !res.status().is_success() {
                debug_response(&res);
                return Err(anyhow!("Error fetching items"));
            }

            let response: GetAttachmentsResponse =
                res.body_json().context("Unable to parse response")?;
            let response_content = response.files;
            let files = response_content.files;

            debug!("Retrieved {} attachments", files.len());
            if !files.is_empty() {
                let retrieved_size = files.len();
                attachments.extend(files);
                if retrieved_size < pagination.page_size {
                    break;
                }

                match pagination.next(response_content.last_id) {
                    Some(p) => pagination = p,
                    None => break,
                }
            } else {
                break;
            }
        }

        debug!(
            "Finished item attachment retrieval process. Retrieved {} attachments",
            attachments.len()
        );

        Ok(attachments)
    }

    pub(crate) async fn fetch_item_revision(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<ItemRevision> {
        let res = self
            .send(GET!(
                "/pass/v1/share/{share_id}/item/{item_id}",
                share_id = share_id,
                item_id = item_id
            ))
            .await
            .context("Error fetching item")?;
        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Invalid status code: {}", res.status()));
        }
        let response: GetItemResponse = res.body_json().context("Error parsing item response")?;

        Ok(response.item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use crate::utils::b64_encode;

    use pass_domain::{
        CustomItem, CustomSection, ItemContent, ItemData, ItemExtraField, ItemExtraFieldContent,
        ItemFlag,
    };

    #[muon_test::test]
    async fn test_fetch_item_revision(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;

        let content = random_string(10);
        let revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(content.clone())
            .build();
        let handled = setup_item_revision(&api, SHARE_ID, ITEM_ID, revision.clone());

        let recorder = api.new_recorder();
        let revision = client
            .fetch_item_revision(&share_id!(SHARE_ID), &item_id!(ITEM_ID))
            .await
            .expect("Should be able to get the item");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        assert_eq!(ITEM_ID, revision.item_id);
        assert_eq!(content, revision.content);
    }

    #[muon_test::test]
    async fn test_view_item(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";
        const ITEM_TITLE: &str = "My item";
        const ITEM_NOTE: &str = "Item note";
        const ITEM_UUID: &str = "1234567890";
        const ITEM_SECTION_NAME: &str = "Section name";
        const ITEM_SECTION_FIELD_TITLE: &str = "Section field title";
        const ITEM_SECTION_FIELD_VALUE: &str = "Section field value";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_vault_share(&api, SHARE_ID);
        setup_share_keys(&api, SHARE_ID);

        let data = ItemData::new(
            ITEM_TITLE.to_string(),
            ITEM_NOTE.to_string(),
            ITEM_UUID.to_string(),
            ItemContent::Custom(CustomItem {
                sections: vec![CustomSection {
                    section_name: ITEM_SECTION_NAME.to_string(),
                    section_fields: vec![ItemExtraField {
                        name: ITEM_SECTION_FIELD_TITLE.to_string(),
                        content: ItemExtraFieldContent::Text(ITEM_SECTION_FIELD_VALUE.to_string()),
                    }],
                }],
            }),
            vec![],
        )
        .expect("Error creating item data");

        let encrypted_data = encrypt_item_contents(data.clone());
        let encoded_data = b64_encode(&encrypted_data.encrypted_contents);
        let revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(encoded_data.clone())
            .with_item_key(Some(b64_encode(encrypted_data.encrypted_item_key.clone())))
            .build();
        let handled = setup_item_revision(&api, SHARE_ID, ITEM_ID, revision.clone());

        let recorder = api.new_recorder();
        let details = client
            .view_item(&share_id!(SHARE_ID), &item_id!(ITEM_ID))
            .await
            .expect("Should be able to view the item");

        assert_hit!(handled);
        let requests = recorder.read();

        assert_eq!(4, requests.len());

        assert_eq!(ITEM_ID, details.item.id.value());
        assert_eq!(data, details.item.content);
    }

    #[muon_test::test]
    async fn test_view_item_fetches_attachments(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_vault_share(&api, SHARE_ID);
        setup_share_keys(&api, SHARE_ID);

        let data = create_random_item();
        let encrypted_data = encrypt_item_contents(data.clone());
        let encoded_data = b64_encode(&encrypted_data.encrypted_contents);
        let revision = ItemRevisionBuilder::new(ITEM_ID.to_string())
            .with_content(encoded_data.clone())
            .with_item_key(Some(b64_encode(encrypted_data.encrypted_item_key.clone())))
            .with_flags(ItemFlag::ItemHasFiles as u64)
            .build();
        setup_item_revision(&api, SHARE_ID, ITEM_ID, revision.clone());

        // TODO: Add test that has attachments to check they can be decrypted
        let handled = api.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{SHARE_ID}/item/{ITEM_ID}/files"),
            move |_| {
                success(GetAttachmentsResponse {
                    files: AttachmentsResponse {
                        files: vec![],
                        last_id: None,
                    },
                })
            },
        );

        let recorder = api.new_recorder();
        client
            .view_item(&share_id!(SHARE_ID), &item_id!(ITEM_ID))
            .await
            .expect("Should be able to view the item");

        assert_hit!(handled);

        let requests = recorder.read();
        assert_eq!(5, requests.len());
    }
}
