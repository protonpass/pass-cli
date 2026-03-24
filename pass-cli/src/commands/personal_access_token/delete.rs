use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_domain::PersonalAccessTokenId;

pub async fn run(client: PassClient, personal_access_token_id: String) -> Result<()> {
    if !pass::is_id(&personal_access_token_id) {
        return Err(anyhow!(
            "Not a valid personal access token id: {}",
            personal_access_token_id
        ));
    }

    client
        .delete_personal_access_token(&PersonalAccessTokenId::new(personal_access_token_id))
        .await
        .context("Error deleting personal access token")?;

    println!("Personal access token deleted successfully");

    Ok(())
}
