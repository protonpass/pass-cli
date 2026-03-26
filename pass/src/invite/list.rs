use crate::PassClient;
use crate::crypto::open_invite_key::OpenInviteKeyFlow;
use crate::permission::PermissionAction;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{Invite, InviteId, InviteVaultData, TargetType, VaultData, crypto};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Clone, Debug, serde::Deserialize)]
struct GetPendingInvitesResponse {
    #[serde(rename = "Invites")]
    pub invites: Vec<PendingInvite>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PendingInvite {
    #[serde(rename = "InviteToken")]
    pub invite_token: String,
    #[serde(rename = "RemindersSent")]
    pub reminders_sent: u8,
    #[serde(rename = "TargetType")]
    pub target_type: u8,
    #[serde(rename = "TargetID")]
    pub target_id: String,
    #[serde(rename = "InviterEmail")]
    pub inviter_email: String,
    #[serde(rename = "InvitedEmail")]
    pub invited_email: String,
    #[serde(rename = "Keys")]
    pub keys: Vec<InviteKeyResponse>,
    #[serde(rename = "VaultData")]
    pub vault_data: Option<PendingInviteVaultData>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct PendingInviteVaultData {
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ContentKeyRotation")]
    pub content_key_rotation: u8,
    #[serde(rename = "MemberCount")]
    pub member_count: u32,
    #[serde(rename = "ItemCount")]
    pub item_count: u32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct InviteKeyResponse {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "KeyRotation")]
    pub key_rotation: u8,
}

#[derive(Clone, Debug)]
pub struct EncryptedInviteKey(pub(crate) Vec<u8>);

#[derive(Clone, Debug)]
pub struct InviteKey {
    pub key: EncryptedInviteKey,
    pub key_rotation: u8,
}

#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub(crate) struct DecryptedInviteKey(pub(crate) Vec<u8>);

impl AsRef<[u8]> for DecryptedInviteKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, ZeroizeOnDrop)]
pub(crate) struct OpenedInviteKey {
    pub key: DecryptedInviteKey,
    pub key_rotation: u8,
}

#[derive(Clone, Debug)]
pub struct InviteWithKeys {
    pub invite: Invite,
    pub keys: Vec<InviteKey>,
}

impl PassClient {
    pub async fn list_user_invites(&self) -> Result<Vec<InviteWithKeys>> {
        self.action_guard(PermissionAction::ListInvites).await?;

        let res = self
            .send(GET!("/pass/v1/invite"))
            .await
            .context("Error fetching invites")?;
        let response: GetPendingInvitesResponse = assert_response!(res);

        let mut result = Vec::new();
        let mut failure_count = 0;
        for invite in response.invites {
            match self.invite_response_to_invite(invite).await {
                Ok(opened) => result.push(opened),
                Err(e) => {
                    warn!("Error opening invite: {e}");
                    failure_count += 1;
                }
            }
        }

        if failure_count > 0 {
            error!("Failed to open {failure_count} invites");
        }

        Ok(result)
    }

    async fn invite_response_to_invite(&self, invite: PendingInvite) -> Result<InviteWithKeys> {
        debug!(
            "[list_invites] invite [{}] | inviter_email [{}] | invited_email: [{}]",
            invite.invite_token, invite.inviter_email, invite.invited_email
        );
        let vault_data = match invite.vault_data {
            None => None,
            Some(data) => {
                let vault_data = self
                    .open_vault_data(
                        &invite.invited_email,
                        &invite.inviter_email,
                        invite.keys.clone(),
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

        Ok(InviteWithKeys {
            invite: Invite {
                id: InviteId::new(invite.invite_token.to_string()),
                token: invite.invite_token,
                target_type: TargetType::from_value(invite.target_type)?,
                target_id: invite.target_id,
                reminders: invite.reminders_sent,
                inviter_email: invite.inviter_email,
                invited_email: invite.invited_email,
                vault_data,
            },
            keys,
        })
    }

    async fn open_vault_data(
        &self,
        invited_address: &str,
        inviter_address: &str,
        keys: Vec<InviteKeyResponse>,
        vault_data: &PendingInviteVaultData,
    ) -> Result<VaultData> {
        let opened_invite_keys = self
            .open_invite_keys(invited_address, inviter_address, keys)
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

    pub(crate) async fn open_invite_vault_data(
        &self,
        invite_key: OpenedInviteKey,
        vault_data: &PendingInviteVaultData,
    ) -> Result<VaultData> {
        let decoded_content = crate::utils::b64_decode(&vault_data.content)
            .context("Error decoding vault_data invite content")?;

        let decrypted = crypto::decrypt(
            &decoded_content,
            invite_key.key.as_ref(),
            crypto::EncryptionTag::VaultContent,
        )
        .map_err(|e| {
            error!("Error decrypting vault data from invite: {e:#}");
            anyhow!("Error decrypting vault data invite")
        })?;

        VaultData::deserialize(&decrypted).context("Error deserializing vault data")
    }

    pub(crate) async fn open_invite_keys(
        &self,
        invited_address: &str,
        inviter_address: &str,
        keys: Vec<InviteKeyResponse>,
    ) -> Result<Vec<OpenedInviteKey>> {
        debug!("[list_invites] Fetching inviter keys [{inviter_address}]");
        let inviter_keys = self
            .get_keys_for_email(inviter_address, true)
            .await
            .context("Error getting keys for inviter")?;

        debug!(
            "[list_invites] Fetched {} keys for inviter",
            inviter_keys.len()
        );

        let invited_addresses = self
            .get_addresses()
            .await
            .context("Error getting addresses")?;
        let invited_address = invited_addresses
            .into_iter()
            .find(|a| a.email == invited_address)
            .ok_or_else(|| anyhow!("Missing invite address"))?;

        let invited_address_keys = self
            .open_address_keys(invited_address.keys)
            .await
            .context("Error opening address keys")?;

        debug!(
            "[list_invites] Get {} keys for invited address",
            invited_address_keys.keys().len()
        );

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
