use crate::PassClient;
use anyhow::{Context, Result};
use async_lock::RwLock;
use pass_domain::{
    Folder, FolderId, Invite, Item, ItemId, Share, ShareId, SyncEventChangedWithToken,
    SyncEventShare, SyncEventShareFolder, SyncEventShareItem, UserEvents,
};
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct EventHandlingResult {
    new_shares: Vec<Share>,
    updated_shares: Vec<Share>,
    deleted_shares: Vec<ShareId>,

    updated_folders: Vec<Folder>,
    deleted_folders: Vec<(ShareId, FolderId)>,

    updated_items: Vec<Item>,
    deleted_items: Vec<(ShareId, ItemId)>,

    new_invites: Vec<Invite>,

    force_refresh: bool,
}

impl PassClient {
    pub async fn on_events(&self, events: UserEvents) -> Result<EventHandlingResult> {
        let result = Arc::new(RwLock::new(EventHandlingResult::default()));

        if events.refresh_user {
            let info = self.get_info().await?;
            info!("Refreshed UserInfo: {info:#?}");
        }

        if events.full_refresh {
            warn!("Should perform full refresh");
        }

        self.handle_organization_info_changed(events.organization_info_changed, result.clone())
            .await
            .context("Error handling organization info changed")?;

        self.handle_shares_created(events.shares_created, result.clone())
            .await
            .context("Error handling shares created")?;
        self.handle_shares_updated(events.shares_updated, result.clone())
            .await
            .context("Error handling shares updated")?;
        self.handle_shares_deleted(events.shares_deleted, result.clone())
            .await
            .context("Error handling shares deleted")?;

        self.handle_folders_updated(events.folders_updated, result.clone())
            .await
            .context("Error handling folders updated")?;
        self.handle_folders_deleted(events.folders_deleted, result.clone())
            .await
            .context("Error handling folders deleted")?;

        self.handle_items_updated(events.items_updated, result.clone())
            .await
            .context("Error handling items updated")?;
        self.handle_items_deleted(events.items_deleted, result.clone())
            .await
            .context("Error handling items deleted")?;
        self.handle_alias_note_changed(events.alias_note_changed, result.clone())
            .await
            .context("Error handling alias note_changed")?;

        self.handle_invites_changed(events.invites_changed, result.clone())
            .await
            .context("Error handling invites changed")?;
        self.handle_group_invites_changed(events.group_invites_changed, result.clone())
            .await
            .context("Error handling group invites changed")?;

        self.handle_breaches_update(events.breach_update, result.clone())
            .await
            .context("Error handling breaches update")?;
        self.handle_pending_alias_to_create(events.pending_alias_to_create_changed, result.clone())
            .await
            .context("Error handling pending alias to create")?;

        let result = result.read().await.clone();

        Ok(result)
    }

    async fn handle_shares_created(
        &self,
        shares: Vec<SyncEventShare>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if shares.is_empty() {
            return Ok(());
        }

        let mut new_shares = Vec::with_capacity(shares.len());
        for share in shares {
            let fetched_share = self
                .get_share(&share.share_id)
                .await
                .context("Error getting share")?;
            new_shares.push(fetched_share);
        }

        result.write().await.new_shares = new_shares;

        Ok(())
    }

    async fn handle_shares_updated(
        &self,
        shares: Vec<SyncEventShare>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if shares.is_empty() {
            return Ok(());
        }

        let mut updated_shares = Vec::with_capacity(shares.len());
        self.clear_shares_cache().await;
        for share in shares {
            let fetched_share = self
                .get_share(&share.share_id)
                .await
                .context("Error getting share")?;
            updated_shares.push(fetched_share);
        }

        result.write().await.updated_shares = updated_shares;

        Ok(())
    }

    async fn handle_shares_deleted(
        &self,
        shares: Vec<SyncEventShare>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if shares.is_empty() {
            return Ok(());
        }

        let mut deleted_shares = Vec::with_capacity(shares.len());
        for share in shares {
            self.clear_items_cache(&share.share_id).await;
            deleted_shares.push(share.share_id);
        }

        result.write().await.deleted_shares = deleted_shares;

        Ok(())
    }

    async fn handle_folders_updated(
        &self,
        folders: Vec<SyncEventShareFolder>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if folders.is_empty() {
            return Ok(());
        }

        let mut updated_folders = Vec::with_capacity(folders.len());
        for folder in folders {
            let folder = self
                .fetch_folder(&folder.share_id, &folder.folder_id)
                .await
                .context("Error getting folder")?;
            updated_folders.push(folder);
        }

        result.write().await.updated_folders = updated_folders;

        Ok(())
    }

    async fn handle_folders_deleted(
        &self,
        folders: Vec<SyncEventShareFolder>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if folders.is_empty() {
            return Ok(());
        }

        let mut deleted_folders = Vec::with_capacity(folders.len());
        for folder in folders {
            deleted_folders.push((folder.share_id, folder.folder_id));
        }

        result.write().await.deleted_folders = deleted_folders;

        Ok(())
    }

    async fn handle_items_updated(
        &self,
        items: Vec<SyncEventShareItem>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut updated_items = Vec::with_capacity(items.len());
        for item in items {
            let updated_item = self
                .view_item(&item.share_id, &item.item_id)
                .await
                .context("Error getting item")?;
            updated_items.push(updated_item.item);
        }

        result.write().await.updated_items = updated_items;

        Ok(())
    }

    async fn handle_items_deleted(
        &self,
        items: Vec<SyncEventShareItem>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        let mut deleted_items = Vec::with_capacity(items.len());
        for item in items {
            deleted_items.push((item.share_id, item.item_id));
        }

        result.write().await.deleted_items = deleted_items;

        Ok(())
    }

    async fn handle_alias_note_changed(
        &self,
        items: Vec<SyncEventShareItem>,
        _result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }

        Ok(())
    }

    async fn handle_invites_changed(
        &self,
        invites: Option<SyncEventChangedWithToken>,
        result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if invites.is_none() {
            return Ok(());
        }

        let invites = self
            .list_user_invites()
            .await
            .context("Error listing user invites")?;
        result.write().await.new_invites = invites.into_iter().map(|i| i.invite).collect();

        Ok(())
    }

    async fn handle_group_invites_changed(
        &self,
        invites: Option<SyncEventChangedWithToken>,
        _result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if invites.is_none() {
            return Ok(());
        }

        Ok(())
    }

    async fn handle_breaches_update(
        &self,
        event: Option<SyncEventChangedWithToken>,
        _result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if event.is_none() {
            return Ok(());
        }

        Ok(())
    }

    async fn handle_pending_alias_to_create(
        &self,
        event: Option<SyncEventChangedWithToken>,
        _result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if event.is_none() {
            return Ok(());
        }

        Ok(())
    }

    async fn handle_organization_info_changed(
        &self,
        event: Option<SyncEventChangedWithToken>,
        _result: Arc<RwLock<EventHandlingResult>>,
    ) -> Result<()> {
        if event.is_none() {
            return Ok(());
        }

        Ok(())
    }
}
