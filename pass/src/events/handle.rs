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
    pub new_shares: Vec<Share>,
    pub updated_shares: Vec<Share>,
    pub deleted_shares: Vec<ShareId>,

    pub updated_folders: Vec<Folder>,
    pub deleted_folders: Vec<(ShareId, FolderId)>,

    pub updated_items: Vec<Item>,
    pub deleted_items: Vec<(ShareId, ItemId)>,

    pub new_invites: Vec<Invite>,

    pub force_refresh: bool,
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

        // Run shares operations in parallel
        let (shares_created_result, shares_updated_result, shares_deleted_result) = futures::join!(
            self.handle_shares_created(events.shares_created, result.clone()),
            self.handle_shares_updated(events.shares_updated, result.clone()),
            self.handle_shares_deleted(events.shares_deleted, result.clone())
        );
        shares_created_result.context("Error handling shares created")?;
        shares_updated_result.context("Error handling shares updated")?;
        shares_deleted_result.context("Error handling shares deleted")?;

        // Run folders operations in parallel (after shares)
        let (folders_updated_result, folders_deleted_result) = futures::join!(
            self.handle_folders_updated(events.folders_updated, result.clone()),
            self.handle_folders_deleted(events.folders_deleted, result.clone())
        );
        folders_updated_result.context("Error handling folders updated")?;
        folders_deleted_result.context("Error handling folders deleted")?;

        // Run items operations in parallel (after folders)
        let (items_updated_result, items_deleted_result, alias_note_result) = futures::join!(
            self.handle_items_updated(events.items_updated, result.clone()),
            self.handle_items_deleted(events.items_deleted, result.clone()),
            self.handle_alias_note_changed(events.alias_note_changed, result.clone())
        );
        items_updated_result.context("Error handling items updated")?;
        items_deleted_result.context("Error handling items deleted")?;
        alias_note_result.context("Error handling alias note_changed")?;

        // Run independent operations in parallel
        let (
            org_info_result,
            invites_result,
            group_invites_result,
            breaches_result,
            pending_alias_result,
        ) = futures::join!(
            self.handle_organization_info_changed(events.organization_info_changed, result.clone()),
            self.handle_invites_changed(events.invites_changed, result.clone()),
            self.handle_group_invites_changed(events.group_invites_changed, result.clone()),
            self.handle_breaches_update(events.breach_update, result.clone()),
            self.handle_pending_alias_to_create(
                events.pending_alias_to_create_changed,
                result.clone()
            )
        );
        org_info_result.context("Error handling organization info changed")?;
        invites_result.context("Error handling invites changed")?;
        group_invites_result.context("Error handling group invites changed")?;
        breaches_result.context("Error handling breaches update")?;
        pending_alias_result.context("Error handling pending alias to create")?;

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

        let futures: Vec<_> = shares
            .into_iter()
            .map(|share| async move {
                self.get_share(&share.share_id)
                    .await
                    .context("Error getting share")
            })
            .collect();

        let new_shares = futures::future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

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

        self.clear_shares_cache().await;

        let futures: Vec<_> = shares
            .into_iter()
            .map(|share| async move {
                self.get_share(&share.share_id)
                    .await
                    .context("Error getting share")
            })
            .collect();

        let updated_shares = futures::future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

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

        let futures: Vec<_> = folders
            .into_iter()
            .map(|folder| async move {
                self.fetch_folder(&folder.share_id, &folder.folder_id)
                    .await
                    .context("Error getting folder")
            })
            .collect();

        let updated_folders = futures::future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

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

        let futures: Vec<_> = items
            .into_iter()
            .map(|item| async move {
                self.view_item(&item.share_id, &item.item_id)
                    .await
                    .context("Error getting item")
                    .map(|view| view.item)
            })
            .collect();

        let updated_items = futures::future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

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
