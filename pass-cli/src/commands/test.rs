use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};

pub async fn run(client: PassClient) -> Result<()> {
    client
        .ping()
        .await
        .context("Error performing connection test")?;
    println!("Connection successful");
    Ok(())
}
