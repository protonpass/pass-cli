use crate::PassClient;
use crate::item::item_keys::OpenedItemKey;
use crate::item::list::ItemRevision;
use crate::pagination::SincePagination;
use crate::utils::debug_response;
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

#[derive(Debug, serde::Deserialize)]
struct GetItemResponse {
    #[serde(rename = "Item")]
    pub item: ItemRevision,
}

#[derive(Debug, serde::Deserialize)]
struct GetAttachmentsResponse {
    #[serde(rename = "Files")]
    pub files: AttachmentsResponse,
}

#[derive(Debug, serde::Deserialize)]
struct AttachmentsResponse {
    #[serde(rename = "Files")]
    pub files: Vec<AttachmentResponse>,
    #[serde(rename = "LastID")]
    pub last_id: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
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
    /*
    #[serde(rename = "RevisionAdded")]
    pub revision_added: i64,
    #[serde(rename = "RevisionRemoved")]
    pub revision_removed: Option<i64>,
    #[serde(rename = "PersistentFileUID")]
    pub persistent_file_uid: String,
    */
}

#[derive(Debug, serde::Deserialize)]
struct ChunkResponse {
    #[serde(rename = "ChunkID")]
    pub chunk_id: String,
    #[serde(rename = "Index")]
    pub index: usize,
    #[serde(rename = "Size")]
    pub size: usize,
}

impl PassClient {
    pub async fn view_item(&self, share_id: &ShareId, item_id: &ItemId) -> Result<ItemDetails> {
        let item_revision = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item")?;

        let mut opened = self
            .open_items(share_id, vec![item_revision])
            .await
            .context("Error opening item")?;
        let (item, item_key) = match opened.pop() {
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
                error!("Error decrypting file key: {}", e);
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
                error!("Error decrypting file metadata: {}", e);
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

            let res = self
                .client
                .send(req)
                .await
                .context("Error fetching items page")?;

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
            .client
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
