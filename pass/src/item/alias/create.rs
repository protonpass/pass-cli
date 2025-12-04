use crate::PassClient;
use crate::item::create::ItemCreatedEvent;
use crate::item::create::common::{CreateItemRequest, CreateItemResponse};
use crate::permission::PermissionAction;
use anyhow::{Context, Result, anyhow};
use muon::POST;
use pass_domain::{AliasItem, ItemContent, ItemId, ItemType, ShareId};

#[derive(Debug)]
pub struct CreatedAliasItem {
    pub alias: String,
    pub item_id: ItemId,
}

#[derive(serde::Serialize)]
struct CreateAliasRequest {
    #[serde(rename = "Prefix")]
    pub prefix: String,
    #[serde(rename = "SignedSuffix")]
    pub signed_suffix: String,
    #[serde(rename = "MailboxIDs")]
    pub mailbox_ids: Vec<i64>,
    #[serde(rename = "AliasName")]
    pub alias_name: Option<String>,
    #[serde(rename = "Item")]
    pub item: CreateItemRequest,
}

impl PassClient {
    pub async fn create_alias(&self, share_id: &ShareId, prefix: &str) -> Result<CreatedAliasItem> {
        self.action_guard(PermissionAction::CreateAlias {
            share_id: share_id.clone(),
        })
        .await?;
        let request = self
            .create_alias_request(share_id, prefix)
            .await
            .context("Error creating create_alias request")?;

        let req = POST!("/pass/v1/share/{share_id}/alias/custom")
            .body_json(request)
            .context("Error serializing create_alias request")?;
        let res = self
            .send(req)
            .await
            .context("Error sending create alias request")?;
        let response: CreateItemResponse = assert_response!(res);

        let email = match response.item.alias_email {
            Some(email) => email,
            None => return Err(anyhow!("Error getting email from created alias")),
        };

        let item_id = ItemId::new(response.item.item_id);

        self.emit_telemetry(&ItemCreatedEvent {
            item_type: ItemType::Alias,
        })
        .await;

        Ok(CreatedAliasItem {
            alias: email,
            item_id,
        })
    }

    async fn create_alias_request(
        &self,
        share_id: &ShareId,
        prefix: &str,
    ) -> Result<CreateAliasRequest> {
        let options = self
            .get_alias_options(share_id)
            .await
            .context("Error fetching alias options")?;

        let suffix = options.suffixes.first().context("No suffix found")?;
        let mailbox = options.mailboxes.first().context("No mailbox found")?;

        let title = format!("Alias for {prefix}");
        let item = self
            .create_item_request(share_id, &title, "", ItemContent::Alias(AliasItem), None)
            .await
            .context("Error creating create_alias_item request")?;

        Ok(CreateAliasRequest {
            prefix: prefix.to_string(),
            signed_suffix: suffix.signed_suffix.to_string(),
            mailbox_ids: vec![mailbox.id],
            alias_name: None,
            item,
        })
    }
}
