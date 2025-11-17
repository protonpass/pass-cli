use anyhow::{Context, Result};
use clap::Subcommand;

#[derive(Subcommand)]
pub enum SupportCommands {
    #[command(about = "Open contact us page in browser")]
    ContactUs,
}

pub async fn run(command: &SupportCommands) -> Result<()> {
    match command {
        SupportCommands::ContactUs => {
            let url = "https://proton.me/support/contact";
            println!("Opening {} in your browser...", url);
            open::that(url)
                .context("Failed to open URL in browser")?;
            Ok(())
        }
    }
}

