use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result};
use muon::PUT;
use pass_domain::ShareId;

#[derive(serde::Deserialize, serde::Serialize)]
struct TransferOwnershipRequest {
    #[serde(rename = "NewOwnerShareID")]
    pub new_owner_share_id: String,
}

impl PassClient {
    pub async fn transfer_ownership(
        &self,
        share_id: &ShareId,
        member_share_id: &ShareId,
    ) -> Result<()> {
        let share = self.get_share(share_id).await?;
        share.vault_share_guard()?;
        share.owner_guard()?;

        let req = PUT!("/pass/v1/vault/{share_id}/owner")
            .body_json(TransferOwnershipRequest {
                new_owner_share_id: member_share_id.to_string(),
            })
            .context("Failed to serialize TransferOwnershipRequest")?;

        let res = self
            .send(req)
            .await
            .context("Error sending transfer ownership request")?;
        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_shares_cache().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    use muon::Method;
    use muon::test::server::Server;
    use pass_domain::{PermissionFlag, TargetType};
    use std::sync::Arc;

    fn setup_vault_share_with_owner(server: &Arc<Server>, share_id: &str, is_owner: bool) {
        let share_response = crate::share::list::ShareResponse {
            share_id: share_id.to_string(),
            address_id: TEST_ADDRESS_ID.to_string(),
            vault_id: TEST_VAULT_ID.to_string(),
            target_type: TargetType::Vault.value(),
            target_id: TEST_VAULT_ID.to_string(),
            owner: is_owner,
            permission: PermissionFlag::Admin.value(),
            share_role_id: if is_owner { "1" } else { "2" }.to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 0,
            group_id: None,
        };
        let share_response_clone = share_response.clone();
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}", share_id),
            move |_| success(share_response_clone.clone()),
        );
        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(crate::share::list::GetSharesResponse {
                shares: vec![share_response.clone()],
            })
        });
    }

    fn setup_item_share(server: &Arc<Server>, share_id: &str) {
        let share_response = crate::share::list::ShareResponse {
            share_id: share_id.to_string(),
            address_id: TEST_ADDRESS_ID.to_string(),
            vault_id: TEST_VAULT_ID.to_string(),
            target_type: TargetType::Item.value(),
            target_id: "TEST_ITEM_ID".to_string(),
            owner: true,
            permission: PermissionFlag::Admin.value(),
            share_role_id: "1".to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 0,
            group_id: None,
        };
        let share_response_clone = share_response.clone();
        server.handler_with_method(
            Method::GET,
            format!("/pass/v1/share/{}", share_id),
            move |_| success(share_response_clone.clone()),
        );
        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(crate::share::list::GetSharesResponse {
                shares: vec![share_response.clone()],
            })
        });
    }

    #[muon::test(scheme(HTTP))]
    async fn test_transfer_ownership_success(server: Arc<Server>) {
        const SHARE_ID: &str = "OwnerShareID";
        const MEMBER_SHARE_ID: &str = "MemberShareID";

        let client = server.pass_client().await;
        setup_vault_share_with_owner(&server, SHARE_ID, true);

        let recorder = server.new_recorder();
        let handled = server.handler_with_method(
            Method::PUT,
            format!("/pass/v1/vault/{}/owner", SHARE_ID),
            |_| success_code(),
        );

        client
            .transfer_ownership(&share_id!(SHARE_ID), &share_id!(MEMBER_SHARE_ID))
            .await
            .expect("Owner should be able to transfer ownership");

        assert_hit!(handled);

        let request: TransferOwnershipRequest = last_request!(recorder);
        assert_eq!(request.new_owner_share_id, MEMBER_SHARE_ID);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_transfer_ownership_not_owner(server: Arc<Server>) {
        const SHARE_ID: &str = "NonOwnerShareID";
        const MEMBER_SHARE_ID: &str = "MemberShareID";

        let client = server.pass_client().await;
        setup_vault_share_with_owner(&server, SHARE_ID, false);

        let handled = server.handler_with_method(
            Method::PUT,
            format!("/pass/v1/vault/{}/owner", SHARE_ID),
            |_| success(Empty),
        );

        let result = client
            .transfer_ownership(&share_id!(SHARE_ID), &share_id!(MEMBER_SHARE_ID))
            .await;

        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("This operation is only valid for owners")
        );

        assert_not_hit!(handled);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_transfer_ownership_item_share(server: Arc<Server>) {
        const SHARE_ID: &str = "ItemShareID";
        const MEMBER_SHARE_ID: &str = "MemberShareID";

        let client = server.pass_client().await;
        setup_item_share(&server, SHARE_ID);

        let handled = server.handler_with_method(
            Method::PUT,
            format!("/pass/v1/vault/{}/owner", SHARE_ID),
            |_| success(Empty),
        );

        let result = client
            .transfer_ownership(&share_id!(SHARE_ID), &share_id!(MEMBER_SHARE_ID))
            .await;

        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("This operation is only valid for vault shares")
        );

        assert_not_hit!(handled);
    }
}
