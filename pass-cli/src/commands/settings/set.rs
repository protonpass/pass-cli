use crate::commands::settings::{SetCommands, Setting};
use crate::commands::vault::VaultQuery;
use crate::helpers::PassClientExt;
use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_db::UserSettingModel;

pub async fn run(subcommand: SetCommands, client: PassClient) -> Result<()> {
    match subcommand {
        SetCommands::DefaultVault {
            vault_name,
            share_id,
        } => set_default_share_id(client, vault_name, share_id).await,
        SetCommands::DefaultFormat { format } => set_default_format(client, format).await,
    }
}

async fn set_default_share_id(
    client: PassClient,
    vault_name: Option<String>,
    share_id: Option<String>,
) -> Result<()> {
    let query = VaultQuery::new(share_id, vault_name)?;
    let share_id = query.resolve(&client).await?;

    let share = client
        .get_share(&share_id)
        .await
        .context("Error getting share")?;
    if share.is_item_share() {
        return Err(anyhow!(
            "Cannot set an item share as the default vault share"
        ));
    }

    let vault_name = match client
        .open_vault_share_content(&share_id, share.content)
        .await
    {
        Ok(content) => content.name,
        Err(e) => return Err(anyhow!("Cannot open vault contents: {e:#}")),
    };

    let client_features = client.get_cli_client_features()?;
    let db = &client_features.database_manager;
    let conn = db.get_connection().await?;

    let user_id = client_features
        .get_user_id()
        .await
        .ok_or_else(|| anyhow!("No active session"))?;

    // Store the share_id
    UserSettingModel::upsert(
        &conn,
        &user_id,
        Setting::DefaultShareId.key(),
        Some(share_id.to_string()),
    )
    .await?;

    println!("Default vault set to {vault_name}: {share_id}");
    Ok(())
}

async fn set_default_format(client: PassClient, format: String) -> Result<()> {
    // Validate format
    let format_lower = format.to_lowercase();
    if format_lower != "human" && format_lower != "json" {
        return Err(anyhow!("Invalid format. Must be 'human' or 'json'"));
    }

    let client_features = client.get_cli_client_features()?;
    let db = &client_features.database_manager;
    let conn = db.get_connection().await?;

    let user_id = client_features
        .get_user_id()
        .await
        .ok_or_else(|| anyhow!("No active session"))?;

    UserSettingModel::upsert(
        &conn,
        &user_id,
        Setting::DefaultFormat.key(),
        Some(format_lower.clone()),
    )
    .await?;

    println!("Default format set to: {}", format_lower);
    Ok(())
}
