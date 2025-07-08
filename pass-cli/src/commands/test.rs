use anyhow::{Context, Result};
use pass::PassClient;

pub async fn run(client: PassClient) -> Result<()> {
    client
        .ping()
        .await
        .context("Error performing connection test")?;
    info!("Connection successful");
    Ok(())
}
