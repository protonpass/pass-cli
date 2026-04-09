use crate::helpers::CliPassClient as PassClient;
use crate::helpers::PassClientExt;
use anyhow::{Result, anyhow};
use clap::Subcommand;
use pass_db::{Setting, UserSettingModel};

#[derive(Subcommand)]
pub enum UnsetCommands {
    #[command(about = "Unset the default vault")]
    DefaultVault,

    #[command(about = "Unset the default output format")]
    DefaultFormat,
}

pub async fn run(subcommand: UnsetCommands, client: PassClient) -> Result<()> {
    let setting = match subcommand {
        UnsetCommands::DefaultVault => Setting::DefaultShareId,
        UnsetCommands::DefaultFormat => Setting::DefaultFormat,
    };

    let client_features = client.get_cli_client_features()?;
    let db = &client_features.database_manager;
    let conn = db.get_connection().await?;

    let user_id = client_features
        .get_user_id()
        .await
        .ok_or_else(|| anyhow!("No active session"))?;

    let deleted = UserSettingModel::delete(&conn, &user_id, setting).await?;

    if deleted > 0 {
        println!("Setting '{}' has been cleared", setting.key());
    } else {
        println!("Setting '{}' was not set", setting.key());
    }

    Ok(())
}
