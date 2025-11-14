use crate::utils::get_base_dir;
use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use tracing::warn;

async fn remove_local_data() -> Result<()> {
    let base_dir = get_base_dir().context("Error getting base dir")?;
    if !base_dir.exists() {
        println!("There was no data to be removed");
        return Ok(());
    }

    if base_dir.is_dir() {
        tokio::fs::remove_dir_all(&base_dir)
            .await
            .context("Error deleting base dir")?;
        Ok(())
    } else {
        Err(anyhow!(
            "Base directory is not a directory: {}",
            base_dir.display()
        ))
    }
}

pub async fn run(client: PassClient) -> Result<()> {
    if let Err(e) = client.logout().await {
        eprintln!("Error logging out: {}", e);
        eprintln!(
            "There has been an error during the logout process. If it persists, you may run 'pass-cli logout --force'"
        );
        std::process::exit(1);
    }

    let key_provider = client
        .get_key_provider()
        .await
        .context("Error getting key provider")?;
    if let Err(e) = key_provider.remove_key().await {
        warn!("Error removing local key: {e:#}");
    }

    remove_local_data().await?;
    println!("Successfully logged out");
    Ok(())
}

pub async fn force_logout() -> Result<()> {
    println!("Executing force logout");
    cleanup().await?;
    println!("Successfully performed force logout");
    Ok(())
}

pub async fn cleanup() -> Result<()> {
    remove_local_data().await?;
    Ok(())
}
