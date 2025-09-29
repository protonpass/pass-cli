use crate::utils::get_base_dir;
use anyhow::{Context, Result, anyhow};
use pass::PassClient;

pub async fn run(client: PassClient) -> Result<()> {
    client.logout().await.context("Error logging out")?;

    let base_dir = get_base_dir().context("Error getting base dir")?;
    if base_dir.is_dir() {
        tokio::fs::remove_dir_all(&base_dir)
            .await
            .context("Error deleting base dir")?;
    } else {
        return Err(anyhow!(
            "Base directory is not a directory: {}",
            base_dir.display()
        ));
    }

    println!("Successfully logged out");
    Ok(())
}

pub async fn force_logout() -> Result<()> {
    println!("Executing force logout");
    let base_dir = get_base_dir().context("Error getting base dir")?;
    if !base_dir.exists() {
        println!("There was no data to be removed");
        return Ok(());
    }

    if base_dir.is_dir() {
        tokio::fs::remove_dir_all(&base_dir)
            .await
            .context("Error deleting base dir")?;

        println!("Successfully performed force logout");
        Ok(())
    } else {
        Err(anyhow!(
            "Base directory is not a directory: {}",
            base_dir.display()
        ))
    }
}
