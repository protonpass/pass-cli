use crate::PassClient;
use crate::common::CodeResponse;
use crate::crypto::reencrypt_invite_keys::{InviteKeyToReencrypt, ReencryptInviteKeysFlow};
use crate::invite::list::InviteWithKeys;
use anyhow::{Context, Result};
use muon::POST;
use pass_domain::InviteId;

#[derive(Debug, serde::Serialize)]
struct AcceptInviteRequest {
    #[serde(rename = "Keys")]
    pub keys: Vec<AcceptInviteKey>,
}

#[derive(Debug, serde::Serialize)]
struct AcceptInviteKey {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
}

impl PassClient {
    pub async fn accept_invite(&self, invite_id: &InviteId) -> Result<()> {
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
            .client
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
                key: k.key.0,
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
                    key: crate::utils::b64_encode(k.encrypted_key),
                    key_rotation: k.key_rotation,
                })
                .collect(),
        })
    }
}
