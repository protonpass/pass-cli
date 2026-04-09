use crate::common::CodeResponse;
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::DELETE;
use pass_domain::InviteId;

impl<C: PassClientContext> PassClient<C> {
    pub async fn reject_invite(&self, invite_id: &InviteId) -> Result<()> {
        self.action_guard(PermissionAction::RejectInvite).await?;

        let res = self
            .send(DELETE!("/pass/v1/invite/{invite_id}"))
            .await
            .context("Error sending reject invite request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}
