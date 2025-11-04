use super::VaultQuery;
use anyhow::{Context, Result};
use pass::{PassClient, UpdateVaultArgs};

pub async fn run(client: PassClient, query: VaultQuery, name: String) -> Result<()> {
    let share_id = query.resolve(&client).await?;
    let args = UpdateVaultArgs::new(name).context("invalid args for update vault")?;
    client
        .update_vault(&share_id, args)
        .await
        .context("error updating vault")?;

    println!("Updated vault");
    Ok(())
}
