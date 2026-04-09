use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::Context;
use pass_domain::{ItemId, ShareId, ShareRole};

impl<C: PassClientContext> PassClient<C> {
    pub async fn share_item(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        email: &str,
        role: &ShareRole,
    ) -> anyhow::Result<()> {
        self.action_guard(PermissionAction::ShareVault).await?;

        let request = self
            .create_invites_request(share_id, email, role, Some(item_id.clone()))
            .await
            .context("Error creating invite to vault request")?;

        self.send_invite(share_id, request)
            .await
            .context("Error sending invite to item request")?;

        Ok(())
    }
}
