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
        let res = self.send(req).await.context("Error downloading chunk")?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::get_one::GetItemResponse;

    use crate::test_tools::*;
    use muon::test::server::{HTTP, Server};
    use pass_domain::{
        CustomItem, CustomSection, ItemAttachmentContent, ItemContent, ItemData, ItemExtraField,
        ItemExtraFieldContent, ItemFlag, ItemState, crypto,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // Helper function to create test attachment data
    fn create_test_attachment(
        attachment_id: &str,
        file_name: &str,
        mime_type: &str,
        chunks: Vec<(String, usize, usize)>, // (chunk_id, index, size)
        encryption_version: u8,
        file_key: &[u8],
        item_key: &[u8],
    ) -> ItemAttachment {
        let content = ItemAttachmentContent {
            name: file_name.to_string(),
            mime_type: mime_type.to_string(),
        };

        let chunks: Vec<AttachmentChunk> = chunks
            .into_iter()
            .map(|(chunk_id, index, size)| AttachmentChunk {
                chunk_id,
                index,
                size,
            })
            .collect();

        // Encrypt the file key with the item key
        let encrypted_file_key =
            crypto::encrypt(file_key, item_key, crypto::EncryptionTag::FileKey)
                .expect("Failed to encrypt file key");

        ItemAttachment {
            id: AttachmentId::new(attachment_id.to_string()),
            size: chunks.iter().map(|c| c.size as u64).sum(),
            encryption_version,
            content,
            chunks,
            encrypted_file_key,
        }
    }

    // Helper function to create encrypted chunk data
    fn create_encrypted_chunk_data(
        data: &[u8],
        file_key: &[u8],
        encryption_version: u8,
        chunk_index: usize,
        num_chunks: usize,
    ) -> Vec<u8> {
        match encryption_version {
            1 => crypto::encrypt(data, file_key, crypto::EncryptionTag::FileData)
                .expect("Failed to encrypt chunk data"),
            2 => crypto::encrypt(
                data,
                file_key,
                crypto::EncryptionTag::FileDataV2 {
                    chunk_index,
                    num_chunks,
                },
            )
            .expect("Failed to encrypt chunk data v2"),
            _ => panic!("Unsupported encryption version: {}", encryption_version),
        }
    }

    // Helper function to create test item data
    fn create_test_item_data(title: &str, note: &str) -> ItemData {
        ItemData::new(
            title.to_string(),
            note.to_string(),
            random_string(10),
            ItemContent::Custom(CustomItem {
                sections: vec![CustomSection {
                    section_name: "Test Section".to_string(),
                    section_fields: vec![ItemExtraField {
                        name: "test_field".to_string(),
                        content: ItemExtraFieldContent::Text("test_value".to_string()),
                    }],
                }],
            }),
            vec![], // extra_fields
        )
        .expect("Failed to create test item data")
    }

    // Helper function to setup item revision with attachment - now returns the encrypted data
    fn setup_item_with_attachment(
        server: &Arc<Server>,
        share_id: &str,
        item_id: &str,
    ) -> crate::test_tools::EncryptItemContentsResult {
        let item_data = create_test_item_data("Test Item", "Test Note");
        let encrypted_data = encrypt_item_contents(item_data);

        let revision = ItemRevisionBuilder::new(item_id.to_string())
            .with_content(crate::utils::b64_encode(&encrypted_data.encrypted_contents))
            .with_item_key(Some(crate::utils::b64_encode(
                &encrypted_data.encrypted_item_key,
            )))
            .with_state(ItemState::Active as u8)
            .with_flags(ItemFlag::ItemHasFiles as u64)
            .build();

        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}/item/{}", share_id, item_id),
            move |_| {
                success(GetItemResponse {
                    item: revision.clone(),
                })
            },
        );

        encrypted_data
    }

    // Helper function to setup chunk download endpoints
    fn setup_chunk_downloads(
        server: &Arc<Server>,
        share_id: &str,
        item_id: &str,
        attachment_id: &str,
        chunk_data: HashMap<String, Vec<u8>>,
    ) {
        for (chunk_id, data) in chunk_data {
            let data_clone = data.clone();
            server.handler_with_method(
                Method::GET,
                format!(
                    "/pass/v1/share/{}/item/{}/file/{}/chunk/{}",
                    share_id, item_id, attachment_id, chunk_id
                ),
                move |_| {
                    Some(
                        muon::test::server::Response::builder()
                            .status(200)
                            .body(axum_core::body::Body::from(data_clone.clone()))
                            .unwrap(),
                    )
                },
            );
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_single_chunk_v1(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const CHUNK_ID: &str = "CHUNK_1";
        const FILE_NAME: &str = "test_file.txt";
        const FILE_CONTENT: &[u8] = b"Hello, World! This is test file content.";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();

        // Create test attachment using the same item key
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            FILE_NAME,
            "text/plain",
            vec![(CHUNK_ID.to_string(), 0, FILE_CONTENT.len())],
            1, // encryption version 1
            &file_key,
            item_key.as_ref(),
        );

        // Setup encrypted chunk data
        let encrypted_chunk_data = create_encrypted_chunk_data(FILE_CONTENT, &file_key, 1, 0, 1);
        let mut chunk_data = HashMap::new();
        chunk_data.insert(CHUNK_ID.to_string(), encrypted_chunk_data);
        setup_chunk_downloads(&server, SHARE_ID, ITEM_ID, ATTACHMENT_ID, chunk_data);

        // Test download
        let downloaded_data = Arc::new(Mutex::new(Vec::new()));
        let downloaded_data_clone = downloaded_data.clone();

        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |data| {
                    let downloaded_data = downloaded_data_clone.clone();
                    async move {
                        downloaded_data.lock().unwrap().extend_from_slice(&data);
                        Ok(())
                    }
                },
            )
            .await;

        assert!(result.is_ok(), "Download should succeed");
        let final_data = downloaded_data.lock().unwrap().clone();
        assert_eq!(
            final_data, FILE_CONTENT,
            "Downloaded data should match original"
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_multiple_chunks_v2(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const FILE_NAME: &str = "large_file.bin";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();

        // Create test data split into chunks
        let chunk1_data = b"First chunk of data. ";
        let chunk2_data = b"Second chunk of data. ";
        let chunk3_data = b"Third and final chunk.";
        let expected_data = [
            chunk1_data.as_slice(),
            chunk2_data.as_slice(),
            chunk3_data.as_slice(),
        ]
        .concat();

        // Create test attachment with multiple chunks
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            FILE_NAME,
            "application/octet-stream",
            vec![
                ("CHUNK_1".to_string(), 0, chunk1_data.len()),
                ("CHUNK_2".to_string(), 1, chunk2_data.len()),
                ("CHUNK_3".to_string(), 2, chunk3_data.len()),
            ],
            2, // encryption version 2
            &file_key,
            item_key.as_ref(),
        );

        // Setup encrypted chunk data with v2 encryption
        let mut chunk_data = HashMap::new();
        chunk_data.insert(
            "CHUNK_1".to_string(),
            create_encrypted_chunk_data(chunk1_data, &file_key, 2, 0, 3),
        );
        chunk_data.insert(
            "CHUNK_2".to_string(),
            create_encrypted_chunk_data(chunk2_data, &file_key, 2, 1, 3),
        );
        chunk_data.insert(
            "CHUNK_3".to_string(),
            create_encrypted_chunk_data(chunk3_data, &file_key, 2, 2, 3),
        );
        setup_chunk_downloads(&server, SHARE_ID, ITEM_ID, ATTACHMENT_ID, chunk_data);

        // Test download with chunks received in order
        let downloaded_chunks = Arc::new(Mutex::new(Vec::new()));
        let downloaded_chunks_clone = downloaded_chunks.clone();

        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |data| {
                    let downloaded_chunks = downloaded_chunks_clone.clone();
                    async move {
                        downloaded_chunks.lock().unwrap().push(data);
                        Ok(())
                    }
                },
            )
            .await;

        assert!(result.is_ok(), "Download should succeed");
        let chunks = downloaded_chunks.lock().unwrap();
        assert_eq!(chunks.len(), 3, "Should receive 3 chunks");

        // Verify chunks are in correct order and content
        assert_eq!(chunks[0], chunk1_data);
        assert_eq!(chunks[1], chunk2_data);
        assert_eq!(chunks[2], chunk3_data);

        // Verify total data
        let total_data: Vec<u8> = chunks.iter().flatten().cloned().collect();
        assert_eq!(total_data, expected_data);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_unordered_chunks(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const FILE_NAME: &str = "unordered_file.txt";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);

        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();

        // Create test data
        let chunk_a_data = b"AAA";
        let chunk_b_data = b"BBB";
        let chunk_c_data = b"CCC";
        let expected_data = [
            chunk_a_data.as_slice(),
            chunk_b_data.as_slice(),
            chunk_c_data.as_slice(),
        ]
        .concat();

        // Create attachment with chunks in non-sequential order
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            FILE_NAME,
            "text/plain",
            vec![
                ("CHUNK_C".to_string(), 2, chunk_c_data.len()), // Index 2
                ("CHUNK_A".to_string(), 0, chunk_a_data.len()), // Index 0
                ("CHUNK_B".to_string(), 1, chunk_b_data.len()), // Index 1
            ],
            1,
            &file_key,
            item_key.as_ref(),
        );

        // Setup encrypted chunk data
        let mut chunk_data = HashMap::new();
        chunk_data.insert(
            "CHUNK_A".to_string(),
            create_encrypted_chunk_data(chunk_a_data, &file_key, 1, 0, 3),
        );
        chunk_data.insert(
            "CHUNK_B".to_string(),
            create_encrypted_chunk_data(chunk_b_data, &file_key, 1, 1, 3),
        );
        chunk_data.insert(
            "CHUNK_C".to_string(),
            create_encrypted_chunk_data(chunk_c_data, &file_key, 1, 2, 3),
        );
        setup_chunk_downloads(&server, SHARE_ID, ITEM_ID, ATTACHMENT_ID, chunk_data);

        // Test download - chunks should be processed in index order despite attachment order
        let downloaded_chunks = Arc::new(Mutex::new(Vec::new()));
        let downloaded_chunks_clone = downloaded_chunks.clone();

        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |data| {
                    let downloaded_chunks = downloaded_chunks_clone.clone();
                    async move {
                        downloaded_chunks.lock().unwrap().push(data);
                        Ok(())
                    }
                },
            )
            .await;

        assert!(result.is_ok(), "Download should succeed");
        let chunks = downloaded_chunks.lock().unwrap();
        assert_eq!(chunks.len(), 3, "Should receive 3 chunks");

        // Verify chunks are processed in correct index order (0, 1, 2)
        assert_eq!(chunks[0], chunk_a_data); // Index 0
        assert_eq!(chunks[1], chunk_b_data); // Index 1
        assert_eq!(chunks[2], chunk_c_data); // Index 2

        let total_data: Vec<u8> = chunks.iter().flatten().cloned().collect();
        assert_eq!(total_data, expected_data);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_chunk_download_error(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const CHUNK_ID: &str = "MISSING_CHUNK";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);
        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();

        // Create test attachment
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            "error_file.txt",
            "text/plain",
            vec![(CHUNK_ID.to_string(), 0, 100)],
            1,
            &file_key,
            item_key.as_ref(),
        );

        // Setup chunk endpoint to return 404
        server.handler_with_method(
            Method::GET,
            format!(
                "/pass/v1/share/{}/item/{}/file/{}/chunk/{}",
                SHARE_ID, ITEM_ID, ATTACHMENT_ID, CHUNK_ID
            ),
            |_| {
                Some(
                    muon::test::server::Response::builder()
                        .status(404)
                        .body(axum_core::body::Body::from(Vec::<u8>::new()))
                        .unwrap(),
                )
            },
        );

        // Test download should fail
        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |_data| async { Ok(()) },
            )
            .await;

        assert!(result.is_err(), "Download should fail for missing chunk");
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("Error downloading chunk"),
            "Error should mention chunk download failure, got: {}",
            error
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_write_callback_error(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const CHUNK_ID: &str = "TEST_CHUNK";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);
        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();
        let test_data = b"Test data";

        // Create test attachment
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            "callback_error.txt",
            "text/plain",
            vec![(CHUNK_ID.to_string(), 0, test_data.len())],
            1,
            &file_key,
            item_key.as_ref(),
        );

        // Setup chunk data
        let encrypted_chunk_data = create_encrypted_chunk_data(test_data, &file_key, 1, 0, 1);
        let mut chunk_data = HashMap::new();
        chunk_data.insert(CHUNK_ID.to_string(), encrypted_chunk_data);
        setup_chunk_downloads(&server, SHARE_ID, ITEM_ID, ATTACHMENT_ID, chunk_data);

        // Test download with failing callback
        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |_data| async { Err(anyhow::anyhow!("Callback failed")) },
            )
            .await;

        assert!(result.is_err(), "Download should fail when callback fails");
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("Error in write callback"),
            "Error should mention callback failure"
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_invalid_encryption_version(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const CHUNK_ID: &str = "TEST_CHUNK";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);
        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();
        let test_data = b"Test data";

        // Create test attachment with invalid encryption version
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            "invalid_version.txt",
            "text/plain",
            vec![(CHUNK_ID.to_string(), 0, test_data.len())],
            99, // Invalid encryption version
            &file_key,
            item_key.as_ref(),
        );

        // Setup chunk data (encrypted with v1 for simplicity)
        let encrypted_chunk_data = create_encrypted_chunk_data(test_data, &file_key, 1, 0, 1);
        let mut chunk_data = HashMap::new();
        chunk_data.insert(CHUNK_ID.to_string(), encrypted_chunk_data);
        setup_chunk_downloads(&server, SHARE_ID, ITEM_ID, ATTACHMENT_ID, chunk_data);

        // Test download should fail
        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |_data| async { Ok(()) },
            )
            .await;

        assert!(
            result.is_err(),
            "Download should fail for invalid encryption version"
        );
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("Error decrypting chunk"),
            "Error should mention chunk decryption failure, got: {}",
            error
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_corrupted_chunk_data(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";
        const CHUNK_ID: &str = "CORRUPTED_CHUNK";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);
        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();

        // Create test attachment
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            "corrupted.txt",
            "text/plain",
            vec![(CHUNK_ID.to_string(), 0, 100)],
            1,
            &file_key,
            item_key.as_ref(),
        );

        // Setup corrupted chunk data (random bytes that can't be decrypted)
        let corrupted_data = vec![0xFF; 100];
        let mut chunk_data = HashMap::new();
        chunk_data.insert(CHUNK_ID.to_string(), corrupted_data);
        setup_chunk_downloads(&server, SHARE_ID, ITEM_ID, ATTACHMENT_ID, chunk_data);

        // Test download should fail
        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |_data| async { Ok(()) },
            )
            .await;

        assert!(
            result.is_err(),
            "Download should fail for corrupted chunk data"
        );
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("Error decrypting chunk"),
            "Error should mention chunk decryption failure"
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_download_attachment_empty_file(server: Arc<Server>) {
        const SHARE_ID: &str = "TEST_SHARE_ID";
        const ITEM_ID: &str = "TEST_ITEM_ID";
        const ATTACHMENT_ID: &str = "TEST_ATTACHMENT_ID";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        setup_share_keys(&server, SHARE_ID);
        // Setup item and get the encrypted data to use the same item key
        let encrypted_data = setup_item_with_attachment(&server, SHARE_ID, ITEM_ID);
        let item_key = encrypted_data.item_key.clone();

        // Generate file key
        let file_key = crypto::generate_encryption_key();

        // Create empty attachment (no chunks)
        let attachment = create_test_attachment(
            ATTACHMENT_ID,
            "empty.txt",
            "text/plain",
            vec![], // No chunks
            1,
            &file_key,
            item_key.as_ref(),
        );

        // Test download of empty file
        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = callback_called.clone();

        let result = client
            .download_attachment(
                &share_id!(SHARE_ID),
                &item_id!(ITEM_ID),
                &attachment,
                |_data| {
                    let callback_called = callback_called_clone.clone();
                    async move {
                        *callback_called.lock().unwrap() = true;
                        Ok(())
                    }
                },
            )
            .await;

        assert!(result.is_ok(), "Download of empty file should succeed");
        assert!(
            !*callback_called.lock().unwrap(),
            "Callback should not be called for empty file"
        );
    }
}
