use crate::protos::file::file_v1;
use anyhow::{Context, Result};

#[derive(Clone, Debug, serde::Serialize)]
pub struct AttachmentId(pub(crate) String);
display_for_basic!(AttachmentId);

impl AttachmentId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct AttachmentChunk {
    pub chunk_id: String,
    pub index: usize,
    pub size: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ItemAttachment {
    pub id: AttachmentId,
    pub size: u64,
    pub encryption_version: u8,
    pub content: ItemAttachmentContent,
    pub chunks: Vec<AttachmentChunk>,
    pub encrypted_file_key: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ItemAttachmentContent {
    pub name: String,
    pub mime_type: String,
}

impl ItemAttachmentContent {
    pub fn serialize(self) -> Result<Vec<u8>> {
        let as_proto = file_v1::FileMetadata::from(self);
        as_proto
            .to_vec()
            .context("Error serializing vault to proto")
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let as_proto = file_v1::FileMetadata::decode_from_slice(data)
            .context("Error decoding FileMetadata from proto")?;
        Ok(Self::from(as_proto))
    }
}

impl From<file_v1::FileMetadata> for ItemAttachmentContent {
    fn from(file_metadata: file_v1::FileMetadata) -> Self {
        Self {
            name: file_metadata.name,
            mime_type: file_metadata.mime_type,
        }
    }
}

impl From<ItemAttachmentContent> for file_v1::FileMetadata {
    fn from(attachment: ItemAttachmentContent) -> Self {
        Self {
            name: attachment.name,
            mime_type: attachment.mime_type,
            ..Default::default()
        }
    }
}
