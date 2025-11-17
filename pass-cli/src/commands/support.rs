use anyhow::{Context, Result};

pub async fn run() -> Result<()> {
    let url = "https://proton.me/support/contact";
    match open::that(url).context("Failed to open URL in browser") {
        Ok(_) => println!("Opening {} in your browser...", url),
        Err(_) => println!(
            "Could not open the browser automatically. Please go to {} to contact us.",
            url
        ),
    };
    Ok(())
}
