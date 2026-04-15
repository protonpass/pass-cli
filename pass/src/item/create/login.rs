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

use super::ItemCreatedEvent;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use pass_domain::{FolderId, ItemContent, ItemId, ItemType, LoginItem, ShareId};

#[derive(Clone, Debug)]
pub struct LoginItemCreatePayload {
    pub title: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub urls: Vec<String>,
    pub totp_uri: Option<String>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn create_login(
        &self,
        share_id: &ShareId,
        payload: LoginItemCreatePayload,
        folder_id: Option<&FolderId>,
    ) -> Result<ItemId> {
        let req = self
            .create_item_request(
                share_id,
                &payload.title,
                "",
                ItemContent::Login(LoginItem {
                    email: payload.email.unwrap_or_default(),
                    username: payload.username.unwrap_or_default(),
                    password: payload.password.unwrap_or_default(),
                    urls: payload.urls,
                    totp_uri: payload.totp_uri.unwrap_or_default(),
                    passkeys: vec![],
                }),
                folder_id,
            )
            .await
            .context("Error creating login item request")?;

        let item_id = self.send_create_item_request(share_id, req).await?;

        self.emit_telemetry(&ItemCreatedEvent {
            item_type: ItemType::Login,
        })
        .await;

        Ok(item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;

    use crate::item::create::common::{CreateItemRequest, CreateItemResponse};
    use crate::item::list::ItemRevision;
    use pass_domain::ItemData;
    use pass_domain::crypto::EncryptionTag;

    #[muon_test::test]
    async fn test_create_login(server: muon_test::Server) {
        let (raw_client, api) = server.client::<()>();
        const ITEM_TITLE: &str = "MyItem";
        const ITEM_EMAIL: &str = "my@item.email.local";
        const ITEM_USERNAME: &str = "MyUsername";
        const ITEM_PASSWORD: &str = "MyPassword";
        const ITEM_WEBSITE: &str = "http://example.local";
        const SHARE_ID: &str = "MyShareID";
        const ITEM_ID: &str = "MyItemID";

        let client = make_test_pass_client_with_setup(raw_client, &api, PlanType::Free).await;
        setup_share_keys(&api, SHARE_ID);
        setup_vault_share(&api, SHARE_ID);

        let handled = api.handler_with_method(
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
                        create_time: 0,
                        modify_time: 0,
                        folder_id: None,
                    },
                })
            },
        );

        let recorder = api.new_recorder();
        let item_id = client
            .create_login(
                &share_id!(SHARE_ID),
                LoginItemCreatePayload {
                    title: ITEM_TITLE.to_string(),
                    email: Some(ITEM_EMAIL.to_string()),
                    username: Some(ITEM_USERNAME.to_string()),
                    password: Some(ITEM_PASSWORD.to_string()),
                    urls: vec![ITEM_WEBSITE.to_string()],
                    totp_uri: None,
                },
                None,
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
