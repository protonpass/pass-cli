use crate::PassClient;
use crate::permission::PermissionAction;
use crate::utils::debug_response;
use anyhow::{Context, Result, anyhow};
use muon::DELETE;
use pass_domain::ShareId;

impl PassClient {
    pub async fn delete_vault(&self, share_id: &ShareId) -> Result<()> {
        self.action_guard(PermissionAction::DeleteVault {
            share_id: share_id.clone(),
        })
        .await?;
        let res = self
            .send(DELETE!("/pass/v1/vault/{}", share_id))
            .await
            .context("Failed to send delete Vault request")?;

        if !res.status().is_success() {
            debug_response(&res);
            return Err(anyhow!("Error in delete Vault request: {}", res.status()));
        }

        self.clear_shares_cache().await;

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
    async fn test_delete_vault(server: Arc<Server>) {
        const SHARE_ID: &str = "MyShareID";

        let client = server.pass_client().await;
        setup_vault_share(&server, SHARE_ID);
        let handled = server.handler_with_method(
            Method::DELETE,
            format!("/pass/v1/vault/{SHARE_ID}"),
            |_| success(Empty),
        );

        client
            .delete_vault(&ShareId::new(SHARE_ID.to_string()))
            .await
            .expect("Should have been able to delete the vault");

        assert_hit!(handled);
    }
}
