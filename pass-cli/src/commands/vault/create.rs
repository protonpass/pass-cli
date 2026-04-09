use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass::CreateVaultArgs;

pub async fn run(client: PassClient, name: String) -> Result<()> {
    let args = CreateVaultArgs::new(name).context("invalid args for create vault")?;
    let (share_id, _) = client
        .create_vault(args)
        .await
        .context("error creating vault")?;

    println!("{share_id}");
    Ok(())
}
