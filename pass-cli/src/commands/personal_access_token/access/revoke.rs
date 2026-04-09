use super::super::PersonalAccessTokenQuery;
use crate::helpers::CliPassClient as PassClient;
use anyhow::{Context, Result};
use pass_domain::ShareId;

pub async fn run(
    client: PassClient,
    query: PersonalAccessTokenQuery,
    share_id: String,
) -> Result<()> {
    let personal_access_token_id = query.resolve(&client).await?;

    client
        .revoke_personal_access_token_access(&personal_access_token_id, &ShareId::new(share_id))
        .await
        .context("Failed to revoke personal access token access")?;

    println!("Personal access token access revoked successfully");

    Ok(())
}
