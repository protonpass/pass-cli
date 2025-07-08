use crate::commands::OutputFormat;
use anyhow::Result;
use clap::Subcommand;
use pass::PassClient;

pub mod accept;
pub mod list;
pub mod reject;

#[derive(Subcommand)]
pub enum InviteCommands {
    #[command(about = "List pending invites")]
    List {
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Accept an invite")]
    Accept {
        #[arg(help = "ID of the invite to accept")]
        invite_id: String,
    },
    #[command(about = "Reject an invite")]
    Reject {
        #[arg(help = "ID of the invite to reject")]
        invite_id: String,
    },
}

pub async fn run(subcommand: InviteCommands, client: PassClient) -> Result<()> {
    match subcommand {
        InviteCommands::List { output } => list::run(client, output).await,
        InviteCommands::Accept { invite_id } => accept::run(client, invite_id).await,
        InviteCommands::Reject { invite_id } => reject::run(client, invite_id).await,
    }
}
