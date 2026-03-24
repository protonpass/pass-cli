use super::super::PersonalAccessTokenQuery;
use crate::commands::Role;
use crate::commands::item::ShareQuery;
use anyhow::{Context, Result, anyhow};
use pass::PassClient;
use pass_domain::{ItemId, ShareRole};

pub async fn run(
    client: PassClient,
    query: PersonalAccessTokenQuery,
    share_id: Option<String>,
    vault_name: Option<String>,
    item_id: Option<String>,
    item_title: Option<String>,
    role: Role,
) -> Result<()> {
    let personal_access_token_id = query.resolve(&client).await?;

    let share_query = ShareQuery::new(share_id, vault_name)?;
    let resolved_share_id = share_query.share_id(&client).await?;

    let resolved_item_id = if let (Some(_id), Some(_title)) = (&item_id, &item_title) {
        return Err(anyhow!("Cannot specify both --item-id and --item-title"));
    } else if let Some(id) = item_id {
        Some(ItemId::new(id))
    } else if let Some(title) = item_title {
        let items = client
            .list_items(&resolved_share_id)
            .await
            .context("Failed to list items")?;

        let item = items
            .iter()
            .find(|i| i.content.title == title)
            .ok_or_else(|| anyhow!("Item not found: {}", title))?;

        Some(item.id.clone())
    } else {
        None
    };

    client
        .grant_personal_access_token_access(
            &personal_access_token_id,
            &resolved_share_id,
            resolved_item_id.as_ref(),
            &ShareRole::from(role),
        )
        .await
        .context("Failed to grant personal access token access")?;

    println!("Personal access token access granted successfully");

    Ok(())
}
