use super::VaultQuery;
use super::key_load::load_and_decrypt_key;
use super::key_storage::{IdentitySource, KeyStorage, SshIdentity};
use anyhow::{Context, Result};
use pass::PassClient;
use pass_domain::{Item, ItemContent, ItemId, ShareId, UserEvents};
use tokio::sync::RwLock;

pub struct SshEventProcessor {
    client: PassClient,
    vault_query: VaultQuery,
    key_storage: KeyStorage,
    vault_share_id_cache: RwLock<Option<ShareId>>,
}

impl SshEventProcessor {
    pub fn new(client: PassClient, vault_query: VaultQuery, key_storage: KeyStorage) -> Self {
        Self {
            client,
            vault_query,
            key_storage,
            vault_share_id_cache: RwLock::new(None),
        }
    }

    pub async fn process_events(&self, events: UserEvents) -> Result<()> {
        debug!("Processing user events");

        let result = self
            .client
            .on_events(events)
            .await
            .context("Failed to process events")?;

        // Process updates (and creates) and deletes
        self.handle_updated_items(result.updated_items).await?;
        self.handle_deleted_items(result.deleted_items).await?;

        Ok(())
    }

    async fn handle_updated_items(&self, items: Vec<Item>) -> Result<()> {
        for item in items {
            // Filter by vault query
            if !self.should_process_event(&item.share_id).await? {
                continue;
            }

            // Filter for SSH keys only
            if let ItemContent::SshKey(ref ssh_key) = item.content.content {
                match load_and_decrypt_key(&item, &ssh_key.private_key) {
                    Ok(private_key) => {
                        match SshIdentity::new(
                            private_key,
                            item.content.title.clone(),
                            IdentitySource::ProtonPass {
                                share_id: item.share_id.clone(),
                                item_id: item.id.clone(),
                            },
                        ) {
                            Ok(identity) => {
                                // Upsert handles both add and update
                                self.key_storage.identity_upsert(identity).await
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create identity for key '{}': {}",
                                    item.content.title, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load/decrypt key '{}': {}", item.content.title, e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_deleted_items(&self, items: Vec<(ShareId, ItemId)>) -> Result<()> {
        for (share_id, item_id) in items {
            // Filter by vault query
            if !self.should_process_event(&share_id).await? {
                continue;
            }

            // Try to remove the identity
            // This will only succeed if it's a ProtonPass key (User keys are preserved)
            if let Err(e) = self
                .key_storage
                .identity_remove_by_item_id(&share_id, &item_id)
                .await
            {
                debug!(
                    "Skipping delete for share_id={}, item_id={}: {}",
                    share_id, item_id, e
                );
            }
        }

        Ok(())
    }

    async fn should_process_event(&self, share_id: &ShareId) -> Result<bool> {
        match self.resolve_vault_query().await? {
            Some(target_share_id) => Ok(target_share_id == *share_id),
            None => Ok(true), // VaultQuery::All
        }
    }

    async fn resolve_vault_query(&self) -> Result<Option<ShareId>> {
        match &self.vault_query {
            VaultQuery::ShareId(id) => Ok(Some(id.clone())),
            VaultQuery::VaultName(name) => {
                // Check cache first
                if let Some(id) = self.vault_share_id_cache.read().await.as_ref() {
                    return Ok(Some(id.clone()));
                }

                // Resolve and cache
                let vault = self
                    .client
                    .find_vault(name)
                    .await
                    .context("Failed to resolve vault name to share_id")?;
                *self.vault_share_id_cache.write().await = Some(vault.share_id.clone());
                Ok(Some(vault.share_id))
            }
            VaultQuery::All => Ok(None), // None means process all
        }
    }
}
