use crate::crypto::encrypt_invite_keys::{EncryptInviteKeysFlow, InviteKeyToPrepare};
use crate::item::item_keys::OpenedItemKeys;
use crate::share::ShareKeys;
use crate::{PassClient, PublicKey};
use anyhow::{Context, Result};
use pass_domain::{Address, ItemId, Share, ShareId, ShareRole, ShareType, TargetType};

pub(crate) enum InviteRequest {
    ExistingUser(CreateInvitesRequest),
    NewUser(NewUserInvitesRequest),
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct NewUserInvitesRequest {
    #[serde(rename = "NewUserInvites")]
    invites: Vec<NewUserInviteRequest>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct NewUserInviteRequest {
    #[serde(rename = "Email")]
    email: String,
    #[serde(rename = "TargetType")]
    target_type: u8,
    #[serde(rename = "Signature")]
    signature: String,
    #[serde(rename = "ShareRoleID")]
    share_role_id: String,
    #[serde(rename = "ItemID")]
    item_id: Option<String>,
    #[serde(rename = "ExpirationTime")]
    expiration_time: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct CreateInvitesRequest {
    #[serde(rename = "Invites")]
    invites: Vec<CreateInviteRequest>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct CreateInviteRequest {
    #[serde(rename = "Keys")]
    keys: Vec<CreateInviteKey>,
    #[serde(rename = "Email")]
    email: String,
    #[serde(rename = "TargetType")]
    target_type: u8,
    #[serde(rename = "ShareRoleID")]
    share_role_id: String,
    #[serde(rename = "Data")]
    data: Option<String>,
    #[serde(rename = "ItemID")]
    item_id: Option<String>,
    #[serde(rename = "ExpirationTime")]
    expiration_time: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct CreateInviteKey {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
}

enum InviteUserMode {
    ExistingUser { keys: Vec<PublicKey> },
    NewUser,
}

enum InviteTarget {
    Vault {
        share_keys: ShareKeys,
    },
    Item {
        item_id: ItemId,
        item_keys: OpenedItemKeys,
    },
}

impl InviteTarget {
    pub fn item_id(&self) -> Option<ItemId> {
        match self {
            Self::Vault { .. } => None,
            Self::Item { item_id, .. } => Some(item_id.clone()),
        }
    }

    pub fn target_type(&self) -> TargetType {
        match self {
            Self::Vault { .. } => TargetType::Vault,
            Self::Item { .. } => TargetType::Item,
        }
    }
}

impl PassClient {
    pub(crate) async fn create_invites_request(
        &self,
        share_id: &ShareId,
        address_to_invite: &str,
        role: &ShareRole,
        item_id: Option<ItemId>,
    ) -> Result<InviteRequest> {
        let share = self
            .get_share(share_id)
            .await
            .context("Error getting share")?;
        share.can_share_guard()?;

        let mode = self
            .get_invite_user_mode(address_to_invite)
            .await
            .context("Error getting invite user mode")?;

        let user_address = self
            .get_address(&share.address_id)
            .await
            .context("Error getting address")?;

        let share_keys = self
            .get_share_keys(share_id)
            .await
            .context("Error getting share keys")?;

        let invite_target = match item_id {
            None => match &share.share_type {
                ShareType::Vault { .. } => {
                    // User with vault access is sharing vault access
                    InviteTarget::Vault { share_keys }
                }
                ShareType::Item { .. } => {
                    // User with item access is trying to share a vault
                    return Err(anyhow::anyhow!(
                        "Share of type item is not allowed to share a vault"
                    ));
                }
            },
            Some(id) => match share.share_type {
                ShareType::Vault { .. } => {
                    // User with vault access is sharing a single item
                    let keys = self
                        .get_item_keys(share_id, &id)
                        .await
                        .context("Error getting item key")?;

                    let opened_keys = self
                        .open_item_keys(share_id, keys)
                        .await
                        .context("Error opening item keys")?;

                    InviteTarget::Item {
                        item_id: id,
                        item_keys: OpenedItemKeys::new(opened_keys),
                    }
                }
                ShareType::Item { ref item_id, .. } => {
                    // User with item access is sharing a single item
                    if !id.eq(item_id) {
                        return Err(anyhow::anyhow!(
                            "Trying to share an item with a share that does not grant access to that item"
                        ));
                    }

                    let key = self
                        .get_item_key_by_ids(share_id, &id)
                        .await
                        .context("Error getting item key")?;
                    InviteTarget::Item {
                        item_id: id,
                        item_keys: OpenedItemKeys::new(vec![key]),
                    }
                }
            },
        };

        match mode {
            InviteUserMode::ExistingUser { keys } => self
                .create_existing_user_invite(
                    &share,
                    user_address,
                    address_to_invite,
                    role,
                    invite_target,
                    keys,
                )
                .await
                .context("Error creating existing user invite"),
            InviteUserMode::NewUser => self
                .create_new_user_invite(
                    &share,
                    user_address,
                    address_to_invite,
                    role,
                    invite_target,
                )
                .await
                .context("Error creating new user invite"),
        }
    }

    async fn create_existing_user_invite(
        &self,
        share: &Share,
        user_address: Address,
        address: &str,
        role: &ShareRole,
        invite_target: InviteTarget,
        invited_keys: Vec<PublicKey>,
    ) -> Result<InviteRequest> {
        let target_type = invite_target.target_type().value();
        let item_id = invite_target.item_id().map(|i| i.value().to_string());
        let encrypted_keys = self
            .encrypt_share_keys_for_user(share, user_address, invite_target, invited_keys)
            .await
            .context("Error encrypting share keys for invited user")?;
        Ok(InviteRequest::ExistingUser(CreateInvitesRequest {
            invites: vec![CreateInviteRequest {
                keys: encrypted_keys,
                email: address.to_string(),
                share_role_id: role.value(),
                expiration_time: None,
                data: None,
                item_id,
                target_type,
            }],
        }))
    }

    async fn create_new_user_invite(
        &self,
        share: &Share,
        user_address: Address,
        address_to_invite: &str,
        role: &ShareRole,
        invite_target: InviteTarget,
    ) -> Result<InviteRequest> {
        let target_type = invite_target.target_type().value();
        let item_id = invite_target.item_id().map(|i| i.value().to_string());
        let key_to_encrypt = match invite_target {
            InviteTarget::Vault { share_keys, .. } => {
                let latest = share_keys.latest_or_err()?;
                let latest_opened = self
                    .open_share_key_for_share(share, latest.clone())
                    .await
                    .context("Error opening share key")?;
                latest_opened.value()
            }
            InviteTarget::Item { item_keys, .. } => {
                let latest = item_keys
                    .latest_or_err()
                    .context("Error getting latest item key")?;
                latest.key.clone().value()
            }
        };

        let signature_body = proton_pass_common::invite::create_signature_body(
            address_to_invite,
            key_to_encrypt.clone(),
        );
        let address_keys = self
            .open_address_keys(user_address.keys)
            .await
            .context("Error opening address keys")?;

        let address_key = address_keys.first_or_err()?;

        let pgp = self.client_features.get_pgp_crypto().await;
        let signed_data = pgp
            .sign(signature_body, address_key.private_key.clone())
            .await
            .context("Error signing new user invite body")?;

        Ok(InviteRequest::NewUser(NewUserInvitesRequest {
            invites: vec![NewUserInviteRequest {
                email: address_to_invite.to_string(),
                signature: crate::utils::b64_encode(signed_data),
                share_role_id: role.value(),
                expiration_time: None,
                target_type,
                item_id,
            }],
        }))
    }

    async fn get_invite_user_mode(&self, address: &str) -> Result<InviteUserMode> {
        let keys = self
            .get_keys_for_email(address, false)
            .await
            .context("Error fetching keys for email")?;

        if keys.is_empty() {
            Ok(InviteUserMode::NewUser)
        } else {
            Ok(InviteUserMode::ExistingUser { keys })
        }
    }
    async fn encrypt_share_keys_for_user(
        &self,
        share: &Share,
        user_address: Address,
        invite_target: InviteTarget,
        invited_keys: Vec<PublicKey>,
    ) -> Result<Vec<CreateInviteKey>> {
        let user_address_keys = self
            .open_address_keys(user_address.keys)
            .await
            .context("Error opening address keys")?;

        let crypto = self.client_features.get_pgp_crypto().await;

        let flow = EncryptInviteKeysFlow::new(crypto, user_address_keys, invited_keys);

        let invite_keys = self
            .prepare_keys_to_invite(share, invite_target)
            .await
            .context("Error preparing keys to invite")?;
        let encrypted_keys = flow
            .encrypt(invite_keys)
            .await
            .context("Error encrypting invite keys")?;

        let keys = encrypted_keys
            .into_iter()
            .map(|k| CreateInviteKey {
                key: crate::utils::b64_encode(k.key.clone()),
                key_rotation: k.key_rotation,
            })
            .collect();

        Ok(keys)
    }

    async fn prepare_keys_to_invite(
        &self,
        share: &Share,
        invite_target: InviteTarget,
    ) -> Result<Vec<InviteKeyToPrepare>> {
        match invite_target {
            InviteTarget::Vault { share_keys, .. } => {
                let mut res = Vec::with_capacity(share_keys.keys.len());
                for key in share_keys.keys {
                    let rotation = key.key_rotation;
                    let opened = self
                        .open_share_key_for_share(share, key)
                        .await
                        .context("Error opening share key")?;
                    res.push(InviteKeyToPrepare {
                        decrypted_key: opened.value(),
                        key_rotation: rotation,
                    });
                }
                Ok(res)
            }
            InviteTarget::Item { item_keys, .. } => Ok(item_keys
                .keys
                .into_iter()
                .map(|k| InviteKeyToPrepare {
                    decrypted_key: k.key.clone().value(),
                    key_rotation: k.key_rotation,
                })
                .collect()),
        }
    }
}
