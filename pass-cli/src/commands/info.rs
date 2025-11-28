use crate::commands::update;
use crate::commands::update::InstallSource;
use crate::telemetry::event::CommandEvent;
use anyhow::{Context, Result};
use pass::PassClient;
use std::path::PathBuf;

pub async fn run(client: PassClient, base_dir: PathBuf) -> Result<()> {
    client.emit_telemetry(&CommandEvent::new("info")).await;
    let info = client.get_info().await.context("Error getting user info")?;

    // Only show ENV if it's not "prod"
    let env_str = format!("{:?}", info.env);
    if env_str != "Prod" {
        println!("- ENV: {}", env_str);
    }

    // Show release track
    let release_track = update::get_release_track(&base_dir)
        .await
        .unwrap_or_else(|_| "stable".to_string());
    println!("- Release track: {}", release_track);

    println!("- ID: {}", info.user.id);
    println!("- Username: {}", info.user.name);
    println!("- Email: {}", info.user.email);

    let install_source = update::get_install_source()?;
    if install_source != InstallSource::Standard {
        println!("- Install source: {:?}", install_source);
    }

    Ok(())
}
