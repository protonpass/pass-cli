use crate::invite::create::{CreateInvitesRequest, InviteRequest, NewUserInvitesRequest};
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::POST;
use pass_domain::{ShareId, ShareRole};

const SUCCESS_CODE: u32 = 1000;

#[derive(Debug, serde::Deserialize)]
struct CreateInvitesResponse {
    #[serde(rename = "Code")]
    code: u32,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn share_vault(
        &self,
        share_id: &ShareId,
        email: &str,
        role: &ShareRole,
    ) -> Result<()> {
        self.action_guard(PermissionAction::ShareVault).await?;

        let request = self
            .create_invites_request(share_id, email, role, None)
            .await
            .context("Error creating invite to vault request")?;

        self.send_invite(share_id, request)
            .await
            .context("Error sending invite request")?;
        Ok(())
    }

    pub(crate) async fn send_invite(
        &self,
        share_id: &ShareId,
        request: InviteRequest,
    ) -> Result<()> {
        match request {
            InviteRequest::ExistingUser(req) => self
                .send_existing_user_invites(share_id, req)
                .await
                .context("Error sending existing user invite")?,
            InviteRequest::NewUser(req) => self
                .send_new_user_invites(share_id, req)
                .await
                .context("Error sending new user invite")?,
        }

        Ok(())
    }

    async fn send_existing_user_invites(
        &self,
        share_id: &ShareId,
        req: CreateInvitesRequest,
    ) -> Result<()> {
        let req = POST!("/pass/v1/share/{share_id}/invite/batch")
            .body_json(req)
            .context("Error creating invites request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending invite request")?;

        let response: CreateInvitesResponse = assert_response!(res);

        if response.code != SUCCESS_CODE {
            return Err(anyhow!(format!("Received invalid code {}", response.code)));
        }

        Ok(())
    }

    async fn send_new_user_invites(
        &self,
        share_id: &ShareId,
        req: NewUserInvitesRequest,
    ) -> Result<()> {
        let req = POST!("/pass/v1/share/{share_id}/invite/new_user/batch")
            .body_json(req)
            .context("Error creating new user invites request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending new user invite request")?;

        let response: CreateInvitesResponse = assert_response!(res);

        if response.code != SUCCESS_CODE {
            return Err(anyhow!(format!("Received invalid code {}", response.code)));
        }

        Ok(())
    }
}
