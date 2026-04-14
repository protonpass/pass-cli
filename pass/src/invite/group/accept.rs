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

use crate::common::CodeResponse;
use crate::crypto::reencrypt_group_invite_keys::{
    GroupInviteKeyToReencrypt, ReencryptGroupInviteKeysFlow,
};
use crate::invite::accept::{AcceptInviteKey, AcceptInviteRequest};
use crate::invite::group::list::GroupInviteWithKeys;
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::POST;
use pass_domain::InviteId;

impl<C: PassClientContext> PassClient<C> {
    pub async fn accept_group_invite(&self, invite_id: &InviteId) -> Result<()> {
        self.action_guard(PermissionAction::AcceptInvite).await?;

        let invites = self
            .list_group_invites()
            .await
            .context("Error getting pending group invites")?;
        let invite = invites
            .into_iter()
            .find(|i| i.invite_with_keys.invite.id.eq(invite_id))
            .ok_or_else(|| anyhow::anyhow!("Group invite not found"))?;

        let request = self
            .accept_group_invite_request(invite)
            .await
            .context("Error creating accept invite request")?;

        let req = POST!("/pass/v1/invite/group/{invite_id}")
            .body_json(request)
            .context("Error creating accept group invite request")?;
        let res = self
            .send(req)
            .await
            .context("Error sending accept group invite request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_shares_cache().await;

        Ok(())
    }

    async fn accept_group_invite_request(
        &self,
        invite: GroupInviteWithKeys,
    ) -> Result<AcceptInviteRequest> {
        let is_group_owner = invite.is_group_owner;
        let invite = invite.invite_with_keys;

        let inviter_keys = self
            .get_keys_for_email(&invite.invite.inviter_email, true)
            .await
            .context("Error getting inviter keys")?;

        let group_addresses = self
            .get_group_addresses()
            .await
            .context("Error getting group address")?;

        let invited_group_address = group_addresses
            .into_iter()
            .find(|a| a.address.email == invite.invite.invited_email)
            .ok_or_else(|| anyhow::anyhow!("Invited Group Address not found"))?;

        let invited_group_address_keys = if is_group_owner {
            // Accepting the invite as a group owner
            self.open_address_keys(invited_group_address.address.keys)
                .await
                .context("Error opening group address keys for owner")?
        } else {
            // Accepting the invite as org admin
            self.open_group_keys(invited_group_address.address.keys)
                .await
                .context("Error opening group address keys for admin")?
        };

        let crypto = self.client_features.get_pgp_crypto().await;
        let flow =
            ReencryptGroupInviteKeysFlow::new(crypto, invited_group_address_keys, inviter_keys);

        let keys_to_reencrypt = invite
            .keys
            .into_iter()
            .map(|k| GroupInviteKeyToReencrypt {
                key: k.key.0.clone(),
                key_rotation: k.key_rotation,
            })
            .collect();

        let reencrypted = flow
            .reencrypt(keys_to_reencrypt)
            .await
            .context("Error reencrypting invite keys")?;

        Ok(AcceptInviteRequest {
            keys: reencrypted
                .into_iter()
                .map(|k| AcceptInviteKey {
                    key: crate::utils::b64_encode(k.encrypted_key.clone()),
                    key_rotation: k.key_rotation,
                })
                .collect(),
        })
    }
}
