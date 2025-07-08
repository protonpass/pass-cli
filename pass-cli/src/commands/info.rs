use anyhow::{Context, Result};
use pass::PassClient;

pub async fn run(client: PassClient) -> Result<()> {
    let info = client.get_info().await.context("Error getting user info")?;
    println!("- ENV: {:?}", info.env);
    println!("- ID: {}", info.user.id);
    println!("- Username: {}", info.user.name);
    println!("- Email: {}", info.user.email);
    Ok(())
}
