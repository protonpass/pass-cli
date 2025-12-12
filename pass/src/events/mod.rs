use crate::PassClient;
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::{
    EventId, FolderId, ItemId, ShareId, SyncEventChangedWithToken, SyncEventShare,
    SyncEventShareFolder, SyncEventShareItem, UserEvents, UserEventsHandler,
};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
struct GetLastEventIdResponse {
    #[serde(rename = "EventID")]
    event_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct SyncEventChangedWithTokenOutput {
    #[serde(rename = "EventToken")]
    pub event_token: String,
}

#[derive(Debug, serde::Deserialize)]
struct SyncEventShareFolderOutput {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "FolderID")]
    pub folder_id: String,
    #[serde(rename = "EventToken")]
    pub event_token: String,
}

#[derive(Debug, serde::Deserialize)]
struct SyncEventShareOutput {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "EventToken")]
    pub event_token: String,
}

#[derive(Debug, serde::Deserialize)]
struct SyncEventShareItemOutput {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "ItemID")]
    pub item_id: String,
    #[serde(rename = "EventToken")]
    pub event_token: String,
}

#[derive(Debug, serde::Deserialize)]
struct Events {
    #[serde(rename = "LastEventID")]
    pub last_event_id: String,
    #[serde(rename = "ItemsUpdated")]
    pub items_updated: Vec<SyncEventShareItemOutput>,
    #[serde(rename = "ItemsDeleted")]
    pub items_deleted: Vec<SyncEventShareItemOutput>,
    #[serde(rename = "AliasNoteChanged")]
    pub alias_note_changed: Vec<SyncEventShareItemOutput>,
    #[serde(rename = "SharesCreated")]
    pub shares_created: Vec<SyncEventShareOutput>,
    #[serde(rename = "SharesUpdated")]
    pub shares_updated: Vec<SyncEventShareOutput>,
    #[serde(rename = "SharesDeleted")]
    pub shares_deleted: Vec<SyncEventShareOutput>,
    #[serde(rename = "FoldersUpdated")]
    pub folders_updated: Vec<SyncEventShareFolderOutput>,
    #[serde(rename = "FoldersDeleted")]
    pub folders_deleted: Vec<SyncEventShareFolderOutput>,
    #[serde(rename = "InvitesChanged")]
    pub invites_changed: Option<SyncEventChangedWithTokenOutput>,
    #[serde(rename = "GroupInvitesChanged")]
    pub group_invites_changed: Option<SyncEventChangedWithTokenOutput>,
    #[serde(rename = "PendingAliasToCreateChanged")]
    pub pending_alias_to_create_changed: Option<SyncEventChangedWithTokenOutput>,
    #[serde(rename = "BreachUpdate")]
    pub breach_update: Option<SyncEventChangedWithTokenOutput>,
    #[serde(rename = "OrganizationInfoChanged")]
    pub organization_info_changed: Option<SyncEventChangedWithTokenOutput>,
    #[serde(rename = "SharesWithInvitesToCreate")]
    pub shares_with_invites_to_create: Vec<SyncEventShareOutput>,
    #[serde(rename = "RefreshUser")]
    pub refresh_user: bool,
    #[serde(rename = "EventsPending")]
    pub events_pending: bool,
    #[serde(rename = "FullRefresh")]
    pub full_refresh: bool,
}

#[derive(Debug, serde::Deserialize)]
struct GetEventsResponse {
    #[serde(rename = "Events")]
    pub events: Events,
}

impl PassClient {
    pub async fn listen_for_events(&self, handler: Arc<dyn UserEventsHandler>) -> Result<()> {
        let initial_event = handler
            .get_last_user_event_id()
            .await
            .context("Error getting last user event id")?;
        let event = match initial_event {
            Some(event) => event,
            None => {
                let event = self
                    .request_last_event_id()
                    .await
                    .context("Error fetching last user event id")?;
                handler
                    .set_last_user_event_id(event.clone())
                    .await
                    .context("Error storing last user event id")?;
                event
            }
        };
        self.start_listening(event, handler)
            .await
            .context("Error while listening for events")
    }

    async fn start_listening(
        &self,
        event_id: EventId,
        handler: Arc<dyn UserEventsHandler>,
    ) -> Result<()> {
        let mut event_id = event_id;
        loop {
            // Fetch new events
            let events = self.fetch_events(&event_id).await?;

            // Check if there are new events
            if events.last_event_id != event_id.value() {
                // Convert API events to domain events
                let user_events = UserEvents::from(events);

                // Tell handler to update the last event ID
                handler
                    .set_last_user_event_id(user_events.last_event_id.clone())
                    .await
                    .context("Error setting last user event id")?;

                // Notify handler with the events
                handler
                    .on_event(user_events.clone())
                    .await
                    .context("Error on events handler")?;

                // Update our local last event ID
                event_id = user_events.last_event_id;
            }

            // Wait until the next time
            handler.tick().await;
        }
    }

    async fn fetch_events(&self, event_id: &EventId) -> Result<Events> {
        let res = self
            .send(GET!("/pass/v1/user/sync_event/{event_id}"))
            .await
            .context("Error fetching events")?;
        let response: GetEventsResponse = assert_response!(res);

        Ok(response.events)
    }

    async fn request_last_event_id(&self) -> Result<EventId> {
        let res = self
            .send(GET!("/pass/v1/user/sync_event"))
            .await
            .context("Error retrieving last user event id")?;
        let response: GetLastEventIdResponse = assert_response!(res);

        Ok(EventId::new(response.event_id))
    }
}

// From implementations to convert API response types to domain types

impl From<SyncEventChangedWithTokenOutput> for SyncEventChangedWithToken {
    fn from(value: SyncEventChangedWithTokenOutput) -> Self {
        Self {
            event_token: EventId::new(value.event_token),
        }
    }
}

impl From<SyncEventShareFolderOutput> for SyncEventShareFolder {
    fn from(value: SyncEventShareFolderOutput) -> Self {
        Self {
            share_id: ShareId::new(value.share_id),
            folder_id: FolderId::new(value.folder_id),
            event_token: EventId::new(value.event_token),
        }
    }
}

impl From<SyncEventShareOutput> for SyncEventShare {
    fn from(value: SyncEventShareOutput) -> Self {
        Self {
            share_id: ShareId::new(value.share_id),
            event_token: EventId::new(value.event_token),
        }
    }
}

impl From<SyncEventShareItemOutput> for SyncEventShareItem {
    fn from(value: SyncEventShareItemOutput) -> Self {
        Self {
            share_id: ShareId::new(value.share_id),
            item_id: ItemId::new(value.item_id),
            event_token: EventId::new(value.event_token),
        }
    }
}

impl From<Events> for UserEvents {
    fn from(value: Events) -> Self {
        Self {
            last_event_id: EventId::new(value.last_event_id),
            items_updated: value
                .items_updated
                .into_iter()
                .map(SyncEventShareItem::from)
                .collect(),
            items_deleted: value
                .items_deleted
                .into_iter()
                .map(SyncEventShareItem::from)
                .collect(),
            alias_note_changed: value
                .alias_note_changed
                .into_iter()
                .map(SyncEventShareItem::from)
                .collect(),
            shares_created: value
                .shares_created
                .into_iter()
                .map(SyncEventShare::from)
                .collect(),
            shares_updated: value
                .shares_updated
                .into_iter()
                .map(SyncEventShare::from)
                .collect(),
            shares_deleted: value
                .shares_deleted
                .into_iter()
                .map(SyncEventShare::from)
                .collect(),
            folders_updated: value
                .folders_updated
                .into_iter()
                .map(SyncEventShareFolder::from)
                .collect(),
            folders_deleted: value
                .folders_deleted
                .into_iter()
                .map(SyncEventShareFolder::from)
                .collect(),
            invites_changed: value.invites_changed.map(SyncEventChangedWithToken::from),
            group_invites_changed: value
                .group_invites_changed
                .map(SyncEventChangedWithToken::from),
            pending_alias_to_create_changed: value
                .pending_alias_to_create_changed
                .map(SyncEventChangedWithToken::from),
            breach_update: value.breach_update.map(SyncEventChangedWithToken::from),
            organization_info_changed: value
                .organization_info_changed
                .map(SyncEventChangedWithToken::from),
            shares_with_invites_to_create: value
                .shares_with_invites_to_create
                .into_iter()
                .map(SyncEventShare::from)
                .collect(),
            refresh_user: value.refresh_user,
            events_pending: value.events_pending,
            full_refresh: value.full_refresh,
        }
    }
}
