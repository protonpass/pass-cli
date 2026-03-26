use crate::PassClient;
use crate::crypto::open_invite_key::OpenInviteKeyFlow;
use crate::invite::list::{
    EncryptedInviteKey, InviteKey, InviteKeyResponse, InviteWithKeys, OpenedInviteKey,
    PendingInviteVaultData,
};
use crate::permission::PermissionAction;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{GroupId, Invite, InviteId, InviteVaultData, TargetType, VaultData};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupInviteContent {
    #[serde(rename = "InviterEmail")]
    pub inviter_email: String,
    #[serde(rename = "InvitedGroupID")]
    pub invited_group_id: String,
    #[serde(rename = "InvitedEmail")]
    pub invited_email: String,
    #[serde(rename = "TargetType")]
    pub target_type: u8,
    #[serde(rename = "RemindersSent")]
    pub reminders_sent: u8,
    #[serde(rename = "InviteToken")]
    pub invite_token: String,
    #[serde(rename = "VaultData")]
    pub vault_data: Option<PendingInviteVaultData>,
    #[serde(rename = "Keys")]
    pub keys: Vec<InviteKeyResponse>,
    #[serde(rename = "IsGroupOwner")]
    pub is_group_owner: bool,
}

#[derive(Clone, Debug)]
pub struct GroupInviteWithKeys {
    pub invite_with_keys: InviteWithKeys,
    pub is_group_owner: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupInvitesResponse {
    #[serde(rename = "Invites")]
    pub invites: Vec<GroupInviteContent>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GetGroupInvitesResponse {
    #[serde(rename = "Invites")]
    pub invites: GroupInvitesResponse,
}

impl PassClient {
    pub async fn list_group_invites(&self) -> Result<Vec<GroupInviteWithKeys>> {
        self.action_guard(PermissionAction::ListInvites).await?;

        let res = self
            .send(GET!("/pass/v1/invite/group"))
            .await
            .context("Error fetching group invites")?;
        let response: GetGroupInvitesResponse = assert_response!(res);

        let mut result = Vec::new();
        for invite in response.invites.invites {
            let opened = self
                .group_invite_response_to_invite(invite)
                .await
                .context("Error opening group invite")?;
            result.push(opened);
        }

        Ok(result)
    }

    async fn group_invite_response_to_invite(
        &self,
        invite: GroupInviteContent,
    ) -> Result<GroupInviteWithKeys> {
        let is_group_owner = invite.is_group_owner;
        let vault_data = match invite.vault_data {
            None => None,
            Some(data) => {
                let vault_data = self
                    .open_vault_data_for_group_invite(
                        &invite.invited_email,
                        &invite.inviter_email,
                        GroupId::new(invite.invited_group_id.clone()),
                        invite.keys.clone(),
                        is_group_owner,
                        &data,
                    )
                    .await
                    .context("Error opening vault data")?;
                Some(InviteVaultData {
                    vault_data,
                    member_count: data.member_count,
                    item_count: data.item_count,
                })
            }
        };

        let mut keys = Vec::with_capacity(invite.keys.len());
        for key in invite.keys {
            let decoded =
                crate::utils::b64_decode(&key.key).context("Error decoding invite key")?;
            keys.push(InviteKey {
                key: EncryptedInviteKey(decoded),
                key_rotation: key.key_rotation,
            });
        }

        Ok(GroupInviteWithKeys {
            invite_with_keys: InviteWithKeys {
                invite: Invite {
                    id: InviteId::new(invite.invite_token.to_string()),
                    token: invite.invite_token,
                    target_type: TargetType::from_value(invite.target_type)?,
                    target_id: "".to_string(), // TODO: Remove
                    reminders: invite.reminders_sent,
                    inviter_email: invite.inviter_email,
                    invited_email: invite.invited_email,
                    vault_data,
                },
                keys,
            },
            is_group_owner,
        })
    }

    async fn open_vault_data_for_group_invite(
        &self,
        invited_address: &str,
        inviter_address: &str,
        invited_group_id: GroupId,
        keys: Vec<InviteKeyResponse>,
        is_group_owner: bool,
        vault_data: &PendingInviteVaultData,
    ) -> Result<VaultData> {
        let opened_invite_keys = self
            .open_group_invite_keys(
                invited_address,
                inviter_address,
                invited_group_id,
                keys,
                is_group_owner,
            )
            .await
            .context("Error opening invite keys")?;

        let invite_key = opened_invite_keys
            .into_iter()
            .find(|k| k.key_rotation == vault_data.content_key_rotation)
            .ok_or_else(|| anyhow!("Missing key rotation"))?;

        self.open_invite_vault_data(invite_key, vault_data)
            .await
            .context("Error opening vault data")
    }

    async fn open_group_invite_keys(
        &self,
        invited_address: &str,
        inviter_address: &str,
        invited_group_id: GroupId,
        keys: Vec<InviteKeyResponse>,
        is_group_owner: bool,
    ) -> Result<Vec<OpenedInviteKey>> {
        let inviter_keys = self
            .get_keys_for_email(inviter_address, true)
            .await
            .context("Error getting keys for inviter")?;

        let group_addresses = self
            .get_group_addresses()
            .await
            .context("Error getting group addresses")?;

        let invited_address_for_group = group_addresses
            .into_iter()
            .find(|a| a.address.email == invited_address && a.group_id == invited_group_id)
            .ok_or_else(|| anyhow!("Missing invited group address"))?;

        let invited_address_keys = if is_group_owner {
            self.open_address_keys(invited_address_for_group.address.keys)
                .await
                .context("Error opening address keys for group owner")?
        } else {
            self.open_group_keys(invited_address_for_group.address.keys)
                .await
                .context("Error opening address keys for group admin")?
        };

        let crypto = self.client_features.get_pgp_crypto().await;
        let flow = OpenInviteKeyFlow::new(crypto, invited_address_keys, inviter_keys);

        let mut invite_keys = Vec::with_capacity(keys.len());
        for key in keys {
            let decoded =
                crate::utils::b64_decode(&key.key).context("Error decoding invite key")?;
            invite_keys.push(InviteKey {
                key: EncryptedInviteKey(decoded),
                key_rotation: key.key_rotation,
            });
        }

        let res = flow
            .open(invite_keys)
            .await
            .context("Error opening invite keys")?;

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use muon::test::server::{HTTP, Server};
    use std::sync::Arc;

    fn make_group_invite_response(is_group_owner: bool) -> GetGroupInvitesResponse {
        GetGroupInvitesResponse {
            invites: GroupInvitesResponse {
                invites: vec![GroupInviteContent {
                    inviter_email: "inviter@example.com".to_string(),
                    invited_group_id: "GroupID1".to_string(),
                    invited_email: "group@example.com".to_string(),
                    target_type: 1,
                    reminders_sent: 0,
                    invite_token: "token123".to_string(),
                    vault_data: None,
                    keys: vec![],
                    is_group_owner,
                }],
            },
        }
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_group_invites_is_group_owner_true(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/pass/v1/invite/group", move |_| {
            success(make_group_invite_response(true))
        });

        let invites = client
            .list_group_invites()
            .await
            .expect("Should list group invites");

        assert_hit!(handled);
        assert_eq!(1, invites.len());
        assert!(invites[0].is_group_owner, "Expected is_group_owner = true");
        assert_eq!("token123", invites[0].invite_with_keys.invite.id.value());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_group_invites_is_group_owner_false(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/pass/v1/invite/group", move |_| {
            success(make_group_invite_response(false))
        });

        let invites = client
            .list_group_invites()
            .await
            .expect("Should list group invites");

        assert_hit!(handled);
        assert_eq!(1, invites.len());
        assert!(
            !invites[0].is_group_owner,
            "Expected is_group_owner = false"
        );
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_group_invites_empty(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/pass/v1/invite/group", move |_| {
            success(GetGroupInvitesResponse {
                invites: GroupInvitesResponse { invites: vec![] },
            })
        });

        let invites = client
            .list_group_invites()
            .await
            .expect("Should list empty group invites");

        assert_hit!(handled);
        assert!(invites.is_empty());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_group_invites_preserves_invite_fields(server: Arc<Server>) {
        let client = server.pass_client().await;

        server.handler_with_method(Method::GET, "/pass/v1/invite/group", move |_| {
            success(GetGroupInvitesResponse {
                invites: GroupInvitesResponse {
                    invites: vec![GroupInviteContent {
                        inviter_email: "alice@example.com".to_string(),
                        invited_group_id: "GID99".to_string(),
                        invited_email: "group99@example.com".to_string(),
                        target_type: 1,
                        reminders_sent: 2,
                        invite_token: "mytoken".to_string(),
                        vault_data: None,
                        keys: vec![],
                        is_group_owner: true,
                    }],
                },
            })
        });

        let invites = client
            .list_group_invites()
            .await
            .expect("Should list group invites");

        assert_eq!(1, invites.len());
        let item = &invites[0];
        assert_eq!("mytoken", item.invite_with_keys.invite.id.value());
        assert_eq!(
            "alice@example.com",
            item.invite_with_keys.invite.inviter_email
        );
        assert_eq!(
            "group99@example.com",
            item.invite_with_keys.invite.invited_email
        );
        assert_eq!(2, item.invite_with_keys.invite.reminders);
        assert!(item.is_group_owner);
    }
}
