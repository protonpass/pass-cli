use crate::PassClient;
use crate::invite::create::InviteRequest;
use anyhow::{Context, Result};
use pass_domain::{InviteId, ShareId, ShareRole};

impl PassClient {
    pub async fn share_vault(
        &self,
        share_id: &ShareId,
        email: &str,
        role: &ShareRole,
    ) -> Result<InviteId> {
        let request = self
            .create_invites_request(share_id, email, role, None)
            .await
            .context("Error creating invite to vault request")?;

        match request {
            InviteRequest::ExistingUser(req) => {
                debug!("Creating existing user invite");
            }
            InviteRequest::NewUser(req) => {
                debug!("Creating new user invite");
            }
        }

        unimplemented!()
    }
}
