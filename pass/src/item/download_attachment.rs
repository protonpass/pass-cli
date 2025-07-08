use crate::PassClient;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{AttachmentChunk, AttachmentId, ItemAttachment, ItemId, ShareId, crypto};
use std::future::Future;

impl PassClient {
    pub async fn download_attachment<F, Fut>(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        attachment: &ItemAttachment,
        mut write_callback: F,
    ) -> Result<()>
    where
        F: FnMut(Vec<u8>) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        debug!(
            "Starting download for attachment: {}",
            attachment.id.value()
        );
        debug!("Attachment has {} chunks", attachment.chunks.len());

        // Sort chunks by index to ensure correct order
        let mut chunks = attachment.chunks.clone();
        chunks.sort_by_key(|chunk| chunk.index);

        let item = self
            .fetch_item_revision(share_id, item_id)
            .await
            .context("Error fetching item revision")?;
        let item_key = self
            .get_item_key(share_id, &item)
            .await
            .context("Error getting item key")?;

        let file_key = crypto::decrypt(
            &attachment.encrypted_file_key,
            item_key.key.as_ref(),
            crypto::EncryptionTag::FileKey,
        )
        .map_err(|e| {
            error!("Error decrypting file key: {}", e);
            anyhow!("Error decrypting file key")
        })?;

        for chunk in chunks {
            debug!(
                "Processing chunk {} (index: {}, size: {})",
                chunk.chunk_id, chunk.index, chunk.size
            );

            let chunk_data = self
                .download_chunk(share_id, item_id, &attachment.id, &chunk.chunk_id)
                .await
                .context("Error downloading chunk")?;

            let decrypted_data = self
                .decrypt_chunk_data(chunk_data, attachment, &chunk, &file_key)
                .await
                .context("Error decrypting chunk")?;

            write_callback(decrypted_data)
                .await
                .context("Error in write callback")?;
        }

        debug!(
            "Download completed for attachment: {}",
            attachment.id.value()
        );
        Ok(())
    }

    async fn download_chunk(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        attachment_id: &AttachmentId,
        chunk_id: &str,
    ) -> Result<Vec<u8>> {
        let req = GET!(
            "/pass/v1/share/{}/item/{}/file/{}/chunk/{}",
            share_id,
            item_id,
            attachment_id,
            chunk_id
        );
        let res = self
            .client
            .send(req)
            .await
            .context("Error downloading chunk")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!(
                "Invalid status code for download chunk: {}",
                res.status()
            ));
        }

        Ok(res.into_body())
    }

    async fn decrypt_chunk_data(
        &self,
        encrypted_data: Vec<u8>,
        attachment: &ItemAttachment,
        chunk: &AttachmentChunk,
        file_key: &[u8],
    ) -> Result<Vec<u8>> {
        let decrypted_body = match attachment.encryption_version {
            1 => crypto::decrypt(&encrypted_data, file_key, crypto::EncryptionTag::FileData),
            2 => crypto::decrypt(
                &encrypted_data,
                file_key,
                crypto::EncryptionTag::FileDataV2 {
                    chunk_index: chunk.index,
                    num_chunks: attachment.chunks.len(),
                },
            ),
            _ => {
                return Err(anyhow!(
                    "Invalid encryption version for download chunk: {}",
                    attachment.encryption_version
                ));
            }
        }
        .map_err(|e| {
            error!("Error decrypting chunk data: {}", e);
            anyhow!("Error decrypting chunk data")
        })?;

        Ok(decrypted_body)
    }
}
