use crate::common::CodeResponse;
use crate::pagination::SincePagination;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::{DELETE, GET, PUT};
use pass_domain::{PermissionFlag, ShareId, ShareMember, ShareRole, TargetType};

#[derive(Debug, serde::Deserialize)]
struct ShareMembersResponse {
    #[serde(rename = "Shares")]
    shares: Vec<ShareMemberResponse>,
    #[serde(rename = "LastToken")]
    last_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ShareMemberResponse {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "UserName")]
    pub user_name: String,
    #[serde(rename = "UserEmail")]
    pub user_email: String,
    #[serde(rename = "Owner")]
    pub owner: bool,
    #[serde(rename = "TargetType")]
    pub target_type: u8,
    #[serde(rename = "Permission")]
    pub permission: u16,
    #[serde(rename = "ShareRoleID")]
    pub share_role_id: String,
    #[serde(rename = "IsGroupShare")]
    pub is_group_share: bool,
}

impl TryFrom<ShareMemberResponse> for ShareMember {
    type Error = anyhow::Error;
    fn try_from(share_response: ShareMemberResponse) -> Result<Self> {
        Ok(Self {
            member_share_id: ShareId::new(share_response.share_id),
            email: share_response.user_email,
            name: share_response.user_name,
            is_group_share: share_response.is_group_share,
            role: ShareRole::from_value(
                &share_response.share_role_id,
                share_response.owner,
                share_response.permission,
            ),
            target_type: TargetType::from_value(share_response.target_type)
                .context("Invalid target type")?,
        })
    }
}

#[derive(serde::Serialize)]
struct UpdateMemberRequest {
    #[serde(rename = "ShareRoleID")]
    share_role_id: String,
    #[serde(rename = "ExpireTime")]
    expire_time: Option<i64>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn list_vault_members(&self, share_id: &ShareId) -> Result<Vec<ShareMember>> {
        let members = self
            .fetch_members(share_id)
            .await
            .context("Error fetching share members")?;

        let mut res = Vec::with_capacity(members.len());
        for member in members {
            res.push(
                ShareMember::try_from(member)
                    .context("Error converting ShareMemberResponse to ShareMember")?,
            );
        }

        Ok(res)
    }

    async fn fetch_members(&self, share_id: &ShareId) -> Result<Vec<ShareMemberResponse>> {
        let mut members = vec![];
        let mut pagination = SincePagination::default();

        loop {
            let mut req = GET!("/pass/v1/share/{}/user", share_id);
            if let Some(ref since) = pagination.since {
                req = req.query(("Since", &since));
            }

            let res = self
                .send(req)
                .await
                .context("Error fetching share members")?;
            let response: ShareMembersResponse = assert_response!(res);

            let should_break = response.shares.len() < pagination.page_size;
            members.extend(response.shares);

            if should_break {
                break;
            }

            pagination = match pagination.next(response.last_token) {
                Some(p) => p,
                None => break,
            };
        }

        Ok(members)
    }

    pub async fn update_vault_member(
        &self,
        share_id: &ShareId,
        member_share_id: &ShareId,
        role: ShareRole,
    ) -> Result<()> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        share.permission_guard(PermissionFlag::Admin)?;

        let request = UpdateMemberRequest {
            share_role_id: role.value(),
            expire_time: None,
        };

        let req = PUT!("/pass/v1/share/{}/user/{}", share_id, member_share_id)
            .body_json(&request)
            .context("Failed to create update member request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send update member request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_shares_cache().await;
        Ok(())
    }

    pub async fn remove_vault_member(
        &self,
        share_id: &ShareId,
        member_share_id: &ShareId,
    ) -> Result<()> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        share.permission_guard(PermissionFlag::Admin)?;

        let res = self
            .send(DELETE!(
                "/pass/v1/share/{}/user/{}",
                share_id,
                member_share_id
            ))
            .await
            .context("Failed to send remove member request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_shares_cache().await;
        Ok(())
    }
}
