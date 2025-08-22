use crate::PassClient;
use crate::item::list::ItemRevision;
use anyhow::{Context, Result};
use muon::POST;
use pass_domain::{ItemContent, ItemId, LoginItem, ShareId};

#[derive(Clone, Debug)]
pub struct LoginItemCreatePayload {
    pub title: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub urls: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CreateItemResponse {
    #[serde(rename = "Item")]
    pub item: ItemRevision,
}

impl PassClient {
    pub async fn create_login(
        &self,
        share_id: &ShareId,
        payload: LoginItemCreatePayload,
    ) -> Result<ItemId> {
        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                ItemContent::Login(LoginItem {
                    email: payload.email.unwrap_or_default(),
                    username: payload.username.unwrap_or_default(),
                    password: payload.password.unwrap_or_default(),
                    urls: payload.urls,
                    totp_uri: String::new(),
                }),
            )
            .await
            .context("Error creating login item request")?;

        let res = POST!("/pass/v1/share/{share_id}/item")
            .body_json(req)
            .context("Error serializing create_login request")?;
        let response = self
            .client
            .send(res)
            .await
            .context("Error sending create login request")?;
        let response: CreateItemResponse = assert_response!(response);

        Ok(ItemId::new(response.item.item_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use crate::item::create::common::CreateItemRequest;
    use muon::test::server::{HTTP, Server};
    use pass_domain::ItemData;
    use pass_domain::crypto::EncryptionTag;

    #[muon::test(scheme(HTTP))]
    async fn test_create_login(server: Arc<Server>) {
        const ITEM_TITLE: &str = "MyItem";
        const ITEM_EMAIL: &str = "my@item.email.local";
        const ITEM_USERNAME: &str = "MyUsername";
        const ITEM_PASSWORD: &str = "MyPassword";
        const ITEM_WEBSITE: &str = "http://example.local";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = server.pass_client().await;
        setup_share_keys(&server, SHARE_ID);
        setup_vault_share(&server, SHARE_ID);

        let handled = server.handler_with_method(
            Method::POST,
            format!("/pass/v1/share/{SHARE_ID}/item"),
            move |_| {
                success(CreateItemResponse {
                    item: ItemRevision {
                        item_id: ITEM_ID.to_string(),
                        revision: 0,
                        content_format_version: 0,
                        key_rotation: 0,
                        content: "".to_string(),
                        item_key: None,
                        state: 0,
                        flags: 0,
                        alias_email: None,
                    },
                })
            },
        );

        let recorder = server.new_recorder();
        let item_id = client
            .create_login(
                &share_id!(SHARE_ID),
                LoginItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    email: Some(ITEM_EMAIL.to_string()),
                    username: Some(ITEM_USERNAME.to_string()),
                    password: Some(ITEM_PASSWORD.to_string()),
                    urls: vec![ITEM_WEBSITE.to_string()],
                },
            )
            .await
            .expect("Should be able to create the item");

        assert_hit!(handled);

        assert_eq!(ITEM_ID, item_id.value());

        let request: CreateItemRequest = last_request!(recorder);

        // Check item is properly encrypted and contains the right contents
        let decoded_encrypted_item_key = crate::utils::b64_decode(&request.item_key).unwrap();
        let decrypted_item_key = pass_domain::crypto::decrypt(
            &decoded_encrypted_item_key,
            &TEST_SHARE_KEY,
            EncryptionTag::ItemKey,
        )
        .expect("Should be able to decrypt item key");
        assert_eq!(32, decrypted_item_key.len());

        let decoded_item_content = crate::utils::b64_decode(&request.content).unwrap();
        let decrypted_item_content = pass_domain::crypto::decrypt(
            &decoded_item_content,
            &decrypted_item_key,
            EncryptionTag::ItemContent,
        )
        .expect("Should be able to decrypt item content");

        let parsed_item_content = ItemData::deserialize(&decrypted_item_content)
            .expect("Should be able to deserialize ItemData");
        assert_eq!(ITEM_TITLE, parsed_item_content.title);

        let login_item = match parsed_item_content.content {
            ItemContent::Login(login_item) => login_item,
            _ => panic!("Should be able a Login"),
        };

        assert_eq!(ITEM_EMAIL, login_item.email);
        assert_eq!(ITEM_USERNAME, login_item.username);
        assert_eq!(ITEM_PASSWORD, login_item.password);
        assert_eq!(1, login_item.urls.len());
        assert_eq!(ITEM_WEBSITE, login_item.urls[0]);
    }
}
