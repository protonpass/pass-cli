use crate::commands::Role;
use anyhow::Result;
use pass::PassClient;

pub async fn run(_client: PassClient, vault_id: String, item_id: String, role: Role) -> Result<()> {
    println!("TODO: Implement item share command");
    println!("Vault ID: {vault_id}, Item ID: {item_id}, Role: {role:?}");
    // TODO: Implement item sharing
    // client.share_item(&vault_id, &item_id, role).await?;
    // println!("Item {} shared with role {:?}", item_id, role);
    Ok(())
}
