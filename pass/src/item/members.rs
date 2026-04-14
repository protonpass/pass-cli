/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use crate::pagination::SincePagination;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::{ItemId, ShareId, ShareMember, ShareRole, TargetType};

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

impl<C: PassClientContext> PassClient<C> {
    pub async fn list_item_members(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<Vec<ShareMember>> {
        let members = self
            .fetch_item_members(share_id, item_id)
            .await
            .context("Error fetching item members")?;

        let mut res = Vec::with_capacity(members.len());
        for member in members {
            res.push(
                ShareMember::try_from(member)
                    .context("Error converting ShareMemberResponse to ShareMember")?,
            );
        }

        Ok(res)
    }

    async fn fetch_item_members(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
    ) -> Result<Vec<ShareMemberResponse>> {
        let mut members = vec![];
        let mut pagination = SincePagination::default();

        loop {
            let mut req = GET!("/pass/v1/share/{}/user/item/{}", share_id, item_id);
            if let Some(ref since) = pagination.since {
                req = req.query(("Since", &since));
            }

            let res = self
                .send(req)
                .await
                .context("Error fetching item members")?;
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
}
