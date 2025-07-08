use anyhow::{Context, Result};
use pass::{PassClient, UpdateVaultArgs};
use pass_domain::ShareId;

pub async fn run(client: PassClient, share_id: ShareId, name: String) -> Result<()> {
    let args = UpdateVaultArgs::new(name).context("invalid args for update vault")?;
    client
        .update_vault(&share_id, args)
        .await
        .context("error updating vault")?;

    println!("Updated vault");
    Ok(())
}
