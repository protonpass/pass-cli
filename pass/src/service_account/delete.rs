use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::Context;
use muon::DELETE;
use pass_domain::ServiceAccountId;

impl PassClient {
    pub async fn delete_service_account(
        &self,
        service_account_id: &ServiceAccountId,
    ) -> anyhow::Result<()> {
        info!("Deleting service account: {service_account_id}");

        let res = self
            .send(DELETE!("/pass/v1/service_account/{service_account_id}"))
            .await
            .context("Failed to delete service account")?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}
