use crate::common::CodeResponse;
use crate::{PassClient, PassClientContext};
use anyhow::Context;
use muon::DELETE;
use pass_domain::PersonalAccessTokenId;

impl<C: PassClientContext> PassClient<C> {
    pub async fn delete_personal_access_token(
        &self,
        personal_access_token_id: &PersonalAccessTokenId,
    ) -> anyhow::Result<()> {
        self.personal_access_token_operation_guard()?;
        info!("Deleting personal access token: {personal_access_token_id}");

        let res = self
            .send(DELETE!(
                "/account/v4/personal-access-token/{personal_access_token_id}"
            ))
            .await
            .context("Failed to delete personal access token")?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}
