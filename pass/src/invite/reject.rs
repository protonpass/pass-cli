use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result};
use muon::DELETE;
use pass_domain::InviteId;

impl PassClient {
    pub async fn reject_invite(&self, invite_id: &InviteId) -> Result<()> {
        let res = self
            .client
            .send(DELETE!("/pass/v1/invite/{invite_id}"))
            .await
            .context("Error sending reject invite request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}
