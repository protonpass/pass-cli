use crate::{PassClient, PassPlan, PlanType};
use anyhow::{Context, Result, anyhow};
use pass_domain::{ItemContent, ItemId, PermissionFlag, ShareId, ShareType};

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermissionAction {
    CreateVault,
    UpdateVault { share_id: ShareId },
    DeleteVault { share_id: ShareId },

    UpdateItem { share_id: ShareId, item_id: ItemId },
    DeleteItem { share_id: ShareId, item_id: ItemId },

    CreateAlias { share_id: ShareId },
    CreateIdentity,
    CreateCreditCard,
}

display_for_enum!(PermissionAction);

impl PassClient {
    pub(crate) async fn action_guard(&self, action: PermissionAction) -> Result<()> {
        let user_access = self
            .get_user_access()
            .await
            .context("Error getting user access data")?;

        match action {
            PermissionAction::CreateVault => self.create_vault_guard(user_access.plan).await,
            PermissionAction::UpdateVault { share_id } => self.update_vault_guard(share_id).await,
            PermissionAction::DeleteVault { share_id } => self.delete_vault_guard(share_id).await,
            PermissionAction::UpdateItem { share_id, item_id } => {
                self.update_item_guard(share_id, item_id).await
            }
            PermissionAction::DeleteItem { share_id, item_id } => {
                self.delete_item_guard(share_id, item_id).await
            }
            PermissionAction::CreateAlias { share_id } => {
                self.create_item_guard(share_id).await?;
                self.create_alias_guard(user_access.plan).await
            }
            PermissionAction::CreateIdentity => self.create_paid_item_guard(user_access.plan).await,
            PermissionAction::CreateCreditCard => {
                self.create_paid_item_guard(user_access.plan).await
            }
        }
    }

    async fn create_vault_guard(&self, plan: PassPlan) -> Result<()> {
        let vault_limit = match plan.vault_limit {
            None => return Ok(()),
            Some(limit) => limit,
        };

        let vaults = self.list_vaults().await.context("Error listing vaults")?;
        let vault_count = vaults.len();
        if vault_count < vault_limit as usize {
            Ok(())
        } else {
            Err(anyhow!(
                "Cannot create a new vault ({vault_count}/{vault_limit})"
            ))
        }
    }

    async fn update_vault_guard(&self, share_id: ShareId) -> Result<()> {
        let share = self
            .get_share(&share_id)
            .await
            .context("Error getting share")?;
        if !share.is_vault_share() {
            return Err(anyhow!("Cannot update vault with a non-vault share"));
        }

        let permission = share.permission;
        if permission.has_flag(PermissionFlag::Admin) || permission.has_flag(PermissionFlag::Update)
        {
            Ok(())
        } else {
            Err(anyhow!("Cannot update vault due to permissions"))
        }
    }

    async fn delete_vault_guard(&self, share_id: ShareId) -> Result<()> {
        let share = self
            .get_share(&share_id)
            .await
            .context("Error getting share")?;
        if !share.is_vault_share() {
            return Err(anyhow!("Cannot delete vault with a non-vault share"));
        }

        let permission = share.permission;
        if permission.has_flag(PermissionFlag::Admin) || permission.has_flag(PermissionFlag::Delete)
        {
            Ok(())
        } else {
            Err(anyhow!("Cannot delete vault due to permissions"))
        }
    }

    async fn create_item_guard(&self, share_id: ShareId) -> Result<()> {
        let share = self
            .get_share(&share_id)
            .await
            .context("Error getting share")?;
        if share.is_item_share() {
            return Err(anyhow!("Cannot create an item with an item share"));
        }

        let permission = share.permission;
        if permission.has_flag(PermissionFlag::Admin) || permission.has_flag(PermissionFlag::Create)
        {
            Ok(())
        } else {
            Err(anyhow!("Cannot create new item due to permissions"))
        }
    }

    async fn create_alias_guard(&self, plan: PassPlan) -> Result<()> {
        let alias_limit = match plan.alias_limit {
            None => return Ok(()),
            Some(limit) => limit,
        };

        let all_shares = self.list_shares().await?;
        let mut all_items = vec![];
        for share in all_shares {
            let items = self
                .list_items(&share.id)
                .await
                .context("Error getting items")?;
            all_items.extend(items);
        }

        let alias_count = all_items
            .iter()
            .filter(|i| matches!(i.content.content, ItemContent::Alias(_)))
            .count();

        if alias_count < alias_limit as usize {
            Ok(())
        } else {
            Err(anyhow!(
                "Cannot create a new alias ({alias_count}/{alias_limit})"
            ))
        }
    }

    async fn update_item_guard(&self, share_id: ShareId, item_id: ItemId) -> Result<()> {
        let share = self
            .get_share(&share_id)
            .await
            .context("Error getting share")?;
        if let ShareType::Item {
            item_id: share_item_id,
            ..
        } = share.share_type
            && share_item_id != item_id
        {
            return Err(anyhow!(
                "Cannot update an item with an item share not for that item"
            ));
        }

        let permission = share.permission;
        if permission.has_flag(PermissionFlag::Admin) || permission.has_flag(PermissionFlag::Update)
        {
            Ok(())
        } else {
            Err(anyhow!("Cannot update item due to permissions"))
        }
    }

    async fn delete_item_guard(&self, share_id: ShareId, item_id: ItemId) -> Result<()> {
        let share = self
            .get_share(&share_id)
            .await
            .context("Error getting share")?;
        if let ShareType::Item {
            item_id: share_item_id,
            ..
        } = share.share_type
            && share_item_id != item_id
        {
            return Err(anyhow!(
                "Cannot delete an item with an item share not for that item"
            ));
        }

        let permission = share.permission;
        if permission.has_flag(PermissionFlag::Admin) || permission.has_flag(PermissionFlag::Delete)
        {
            Ok(())
        } else {
            Err(anyhow!("Cannot delete item due to permissions"))
        }
    }

    async fn create_paid_item_guard(&self, plan: PassPlan) -> Result<()> {
        match plan.type_ {
            PlanType::Free => Err(anyhow!(
                "Your plan does not include creating this type of item"
            )),
            PlanType::Plus | PlanType::Business => Ok(()),
        }
    }
}
