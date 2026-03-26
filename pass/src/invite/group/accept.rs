use crate::PassClient;
use crate::common::CodeResponse;
use crate::crypto::reencrypt_group_invite_keys::{
    GroupInviteKeyToReencrypt, ReencryptGroupInviteKeysFlow,
};
use crate::invite::accept::{AcceptInviteKey, AcceptInviteRequest};
use crate::invite::group::list::GroupInviteWithKeys;
use crate::permission::PermissionAction;
use anyhow::{Context, Result};
use muon::POST;
use pass_domain::InviteId;

impl PassClient {
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
