/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::helpers::CliPassClient as PassClient;
use anyhow::Result;
use clap::Subcommand;
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
