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
