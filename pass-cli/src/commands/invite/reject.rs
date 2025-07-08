use anyhow::Result;
use pass::PassClient;

pub async fn run(_client: PassClient, invite_id: String) -> Result<()> {
    println!("TODO: Implement invite reject command for invite: {invite_id}");
    // TODO: Implement invite rejection
    // client.reject_invite(&invite_id).await?;
    // println!("Invite {} rejected successfully", invite_id);
    Ok(())
}
