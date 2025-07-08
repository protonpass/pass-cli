use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;
use std::path::PathBuf;

pub mod download;

#[derive(Subcommand)]
pub enum AttachmentCommands {
    #[command(about = "Download an attachment")]
    Download {
        #[arg(long, help = "Share ID of the vault containing the item")]
        share_id: String,
        #[arg(long, help = "ID of the item containing the attachment")]
        item_id: String,
        #[arg(long, help = "ID of the attachment to download")]
        attachment_id: String,
        #[arg(long, help = "Output path for the downloaded attachment")]
        output: PathBuf,
    },
}

pub async fn run(subcommand: AttachmentCommands, client: PassClient) -> Result<()> {
    match subcommand {
        AttachmentCommands::Download {
            share_id,
            item_id,
            attachment_id,
            output,
        } => download::run(client, share_id, item_id, attachment_id, output).await,
    }
}
