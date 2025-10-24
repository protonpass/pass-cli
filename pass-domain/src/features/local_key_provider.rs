use anyhow::Result;

#[async_trait::async_trait]
pub trait LocalKeyProvider: Send + Sync {
    async fn get_key(&self) -> Result<Vec<u8>>;
    async fn remove_key(&self) -> Result<()>;
}
