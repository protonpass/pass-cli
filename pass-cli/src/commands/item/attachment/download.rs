use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::{ItemId, ShareId};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn run(
    client: PassClient,
    share_id: String,
    item_id: String,
    attachment_id: String,
    output: PathBuf,
) -> Result<()> {
    println!("Downloading attachment...");
    println!("Share ID: {share_id}");
    println!("Item ID: {item_id}");
    println!("Attachment ID: {attachment_id}");
    println!("Output path: {}", output.display());

    let share_id = ShareId::new(share_id);
    let item_id = ItemId::new(item_id);

    // Get the item with attachments
    let item = client
        .view_item(&share_id, &item_id)
        .await
        .context("Error retrieving item")?;

    // Find the attachment
    let attachment = item
        .attachments
        .into_iter()
        .find(|att| att.id.value() == attachment_id)
        .context("Attachment not found")?;

    println!("Found attachment: {}", attachment.content.name);
    println!("Attachment size: {} bytes", attachment.size);
    println!("Attachment type: {}", attachment.content.mime_type);
    println!("Attachment chunks: {}", attachment.chunks.len());

    // Create or truncate the output file
    let file = tokio::fs::File::create(&output)
        .await
        .context("Error creating output file")?;
    let file = Arc::new(Mutex::new(file));

    let file_clone = file.clone();
    // Download attachment with callback to write chunks
    client
        .download_attachment(
            &share_id,
            &item_id,
            &attachment,
            move |chunk_data: Vec<u8>| {
                let file = file_clone.clone();
                async move {
                    use tokio::io::AsyncWriteExt;
                    let mut file = file.lock().await;
                    file.write_all(&chunk_data)
                        .await
                        .context("Error writing chunk to file")
                }
            },
        )
        .await
        .context("Error downloading attachment")?;

    println!(
        "Successfully downloaded attachment to: {}",
        output.display()
    );

    Ok(())
}
