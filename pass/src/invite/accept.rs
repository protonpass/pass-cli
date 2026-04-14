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
use crate::crypto::reencrypt_invite_keys::{InviteKeyToReencrypt, ReencryptInviteKeysFlow};
use crate::invite::list::InviteWithKeys;
use crate::permission::PermissionAction;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::POST;
use pass_domain::InviteId;

#[derive(Debug, serde::Serialize)]
pub(crate) struct AcceptInviteRequest {
    #[serde(rename = "Keys")]
    pub keys: Vec<AcceptInviteKey>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct AcceptInviteKey {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn accept_invite(&self, invite_id: &InviteId) -> Result<()> {
        self.action_guard(PermissionAction::AcceptInvite).await?;

        let invites = self
            .list_user_invites()
            .await
            .context("Error getting pending invites")?;
        let invite = invites
            .into_iter()
            .find(|i| i.invite.id.eq(invite_id))
            .ok_or_else(|| anyhow::anyhow!("Invite not found"))?;

        let request = self
            .accept_invite_request(invite)
            .await
            .context("Error creating accept invite request")?;

        let req = POST!("/pass/v1/invite/{invite_id}")
            .body_json(request)
            .context("Error creating accept invite request")?;
        let res = self
            .send(req)
            .await
            .context("Error sending accept invite request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        self.clear_shares_cache().await;

        Ok(())
    }

    async fn accept_invite_request(&self, invite: InviteWithKeys) -> Result<AcceptInviteRequest> {
        let inviter_keys = self
            .get_keys_for_email(&invite.invite.inviter_email, true)
            .await
            .context("Error getting inviter keys")?;

        let user_keys = self
            .get_user_keys()
            .await
            .context("Error getting user keys")?;
        let addresses = self
            .get_addresses()
            .await
            .context("Error getting addresses")?;
        let address = addresses
            .into_iter()
            .find(|a| a.email == invite.invite.invited_email)
            .ok_or_else(|| anyhow::anyhow!("Invited address not found"))?;

        let address_keys = self
            .open_address_keys(address.keys)
            .await
            .context("Error opening address keys")?;

        let crypto = self.client_features.get_pgp_crypto().await;
        let flow = ReencryptInviteKeysFlow::new(crypto, user_keys, address_keys, inviter_keys);

        let keys_to_reencrypt = invite
            .keys
            .into_iter()
            .map(|k| InviteKeyToReencrypt {
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
