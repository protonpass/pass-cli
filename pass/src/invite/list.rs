use crate::PassClient;
use crate::crypto::open_invite_key::OpenInviteKeyFlow;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{Invite, InviteId, InviteVaultData, TargetType, VaultData, crypto};

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
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PendingInviteVaultData {
    #[serde(rename = "Content")]
    pub content: String,
    #[serde(rename = "ContentKeyRotation")]
    pub content_key_rotation: u8,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: u8,
    #[serde(rename = "MemberCount")]
    pub member_count: u32,
    #[serde(rename = "ItemCount")]
    pub item_count: u32,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct InviteKeyResponse {
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

#[derive(Clone, Debug)]
pub(crate) struct DecryptedInviteKey(pub(crate) Vec<u8>);

impl AsRef<[u8]> for DecryptedInviteKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OpenedInviteKey {
    pub key: DecryptedInviteKey,
    pub key_rotation: u8,
}

impl PassClient {
    pub async fn list_user_invites(&self) -> Result<Vec<Invite>> {
        let res = self
            .client
            .send(GET!("/pass/v1/invite"))
            .await
            .context("Error fetching invites")?;
        let response: GetPendingInvitesResponse = assert_response!(res);

        let mut result = Vec::new();
        for invite in response.invites {
            let opened = self
                .invite_response_to_invite(invite)
                .await
                .context("Error opening invite")?;
            result.push(opened);
        }

        Ok(result)
    }

    async fn invite_response_to_invite(&self, invite: PendingInvite) -> Result<Invite> {
        let vault_data = match invite.vault_data {
            None => None,
            Some(data) => {
                let vault_data = self
                    .open_vault_data(
                        &invite.invited_email,
                        &invite.inviter_email,
                        invite.keys,
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

        Ok(Invite {
            id: InviteId::new(invite.invite_token.to_string()),
            token: invite.invite_token,
            target_type: TargetType::from_value(invite.target_type)?,
            target_id: invite.target_id,
            reminders: invite.reminders_sent,
            inviter_email: invite.inviter_email,
            invited_email: invite.invited_email,
            vault_data,
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

        let decoded_content = crate::utils::b64_decode(&vault_data.content)
            .context("Error decoding vault_data invite content")?;

        let decrypted = crypto::decrypt(
            &decoded_content,
            invite_key.key.as_ref(),
            crypto::EncryptionTag::VaultContent,
        )
        .map_err(|e| {
            error!("Error decrypting vault data from invite: {}", e);
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
        let inviter_keys = self
            .get_keys_for_email(inviter_address, true)
            .await
            .context("Error getting keys for inviter")?;

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
