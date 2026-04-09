mod accept;
mod list;

use crate::commands::OutputFormat;
use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;
use pass_domain::InviteId;

#[derive(Subcommand)]
pub enum GroupInviteCommands {
    #[command(about = "List pending invites")]
    List {
        #[arg(long, default_value = "human")]
        output: OutputFormat,
    },
    #[command(about = "Accept group invite")]
    Accept { invite_id: String },
}

pub async fn run(command: GroupInviteCommands, client: PassClient) -> Result<()> {
    match command {
        GroupInviteCommands::List { output } => list::run(client, output).await,
        GroupInviteCommands::Accept { invite_id } => {
            accept::run(client, InviteId::new(invite_id)).await
        }
    }
}
