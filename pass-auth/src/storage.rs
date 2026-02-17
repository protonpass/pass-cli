use anyhow::Result;

#[async_trait::async_trait]
pub trait SessionStorage: Send + Sync {
    async fn load(&self) -> Result<Option<Vec<u8>>>;
    async fn save(&self, data: &[u8]) -> Result<()>;
    async fn exists(&self) -> bool;
    async fn delete(&self) -> Result<()>;
}
