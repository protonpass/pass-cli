use crate::{TargetType, VaultData};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct InviteId(pub(crate) String);
display_for_basic!(InviteId);

impl InviteId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Invite {
    pub id: InviteId,
    pub token: String,
    pub target_type: TargetType,
    pub target_id: String,
    pub reminders: u8,
    pub inviter_email: String,
    pub invited_email: String,
    pub vault_data: Option<InviteVaultData>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct InviteVaultData {
    pub vault_data: VaultData,
    pub member_count: u32,
    pub item_count: u32,
}
