use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result};
use muon::DELETE;
use pass_domain::{ServiceAccountId, ShareId};

impl PassClient {
    pub async fn revoke_service_account_access(
        &self,
        service_account_id: &ServiceAccountId,
        share_id: &ShareId,
    ) -> Result<()> {
        info!("Revoking service account {service_account_id} access from share {share_id}");

        let res = self
            .send(DELETE!(
                "/pass/v1/service_account/{}/access/{}",
                service_account_id,
                share_id.value()
            ))
            .await
            .context("Failed to revoke service account access")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        info!(
            "Service account {} access revoked successfully from share {}",
            service_account_id, share_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_revoke_access(server: Arc<Server>) {
        const SERVICE_ACCOUNT_ID: &str = "test_sa_id";
        const SHARE_ID: &str = "test_share_id";
        const REVOKE_PATH: &str = "/pass/v1/service_account/test_sa_id/access/test_share_id";

        let client = server.pass_client().await;

        let revoke_handled =
            server.handler_with_method(Method::DELETE, REVOKE_PATH, |_| success_code());

        client
            .revoke_service_account_access(
                &ServiceAccountId::new(SERVICE_ACCOUNT_ID.to_string()),
                &ShareId::new(SHARE_ID.to_string()),
            )
            .await
            .expect("Should be able to revoke access");

        assert_hit!(revoke_handled);
    }
}
