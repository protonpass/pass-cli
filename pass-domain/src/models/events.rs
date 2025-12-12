use crate::{EventId, FolderId, ItemId, ShareId};

#[derive(Clone, Debug)]
pub struct SyncEventChangedWithToken {
    pub event_token: EventId,
}

#[derive(Clone, Debug)]
pub struct SyncEventShareFolder {
    pub share_id: ShareId,
    pub folder_id: FolderId,
    pub event_token: EventId,
}

#[derive(Clone, Debug)]
pub struct SyncEventShare {
    pub share_id: ShareId,
    pub event_token: EventId,
}

#[derive(Clone, Debug)]
pub struct SyncEventShareItem {
    pub share_id: ShareId,
    pub item_id: ItemId,
    pub event_token: EventId,
}

#[derive(Clone, Debug)]
pub struct UserEvents {
    pub last_event_id: EventId,
    pub items_updated: Vec<SyncEventShareItem>,
    pub items_deleted: Vec<SyncEventShareItem>,
    pub alias_note_changed: Vec<SyncEventShareItem>,
    pub shares_created: Vec<SyncEventShare>,
    pub shares_updated: Vec<SyncEventShare>,
    pub shares_deleted: Vec<SyncEventShare>,
    pub folders_updated: Vec<SyncEventShareFolder>,
    pub folders_deleted: Vec<SyncEventShareFolder>,
    pub invites_changed: Option<SyncEventChangedWithToken>,
    pub group_invites_changed: Option<SyncEventChangedWithToken>,
    pub pending_alias_to_create_changed: Option<SyncEventChangedWithToken>,
    pub breach_update: Option<SyncEventChangedWithToken>,
    pub organization_info_changed: Option<SyncEventChangedWithToken>,
    pub shares_with_invites_to_create: Vec<SyncEventShare>,
    pub refresh_user: bool,
    pub events_pending: bool,
    pub full_refresh: bool,
}
