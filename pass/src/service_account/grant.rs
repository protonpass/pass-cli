use crate::PassClient;
use crate::common::CodeResponse;
use anyhow::{Context, Result, anyhow};
use muon::POST;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ItemId, ShareId, ShareRole, TargetType, crypto};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct KeyRotationKeyPair {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "KeyRotation")]
    key_rotation: u8,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct ServiceAccountGrantAccessRequest {
    #[serde(rename = "ShareID")]
    share_id: String,
    #[serde(rename = "TargetID", skip_serializing_if = "Option::is_none")]
    target_id: Option<String>,
    #[serde(rename = "TargetType")]
    target_type: u8,
    #[serde(rename = "ShareRoleID")]
    share_role_id: String,
    #[serde(rename = "Keys")]
    keys: Vec<KeyRotationKeyPair>,
    #[serde(rename = "ExpireTime", skip_serializing_if = "Option::is_none")]
    expire_time: Option<i64>,
}

impl PassClient {
    pub async fn grant_service_account_access(
        &self,
        service_account_id: &str,
        share_id: &ShareId,
        item_id: Option<&ItemId>,
        role: &ShareRole,
        expiration_time: Option<i64>,
    ) -> Result<()> {
        info!(
            "Granting service account {} access to share {} (item: {:?})",
            service_account_id, share_id, item_id
        );

        // Prepare the access grant request
        let request = self
            .prepare_grant_access_request(
                service_account_id,
                share_id,
                item_id,
                role,
                expiration_time,
            )
            .await?;

        // Send the request to the server
        self.send_grant_access_request(service_account_id, request)
            .await?;

        info!(
            "Service account {} granted access successfully",
            service_account_id
        );

        Ok(())
    }

    async fn prepare_grant_access_request(
        &self,
        service_account_id: &str,
        share_id: &ShareId,
        item_id: Option<&ItemId>,
        role: &ShareRole,
        expiration_time: Option<i64>,
    ) -> Result<ServiceAccountGrantAccessRequest> {
        let service_account_key = self
            .get_service_account_key(service_account_id)
            .await
            .context("Failed to get service account key")?;

        let (target_type, target_id, keys) = if let Some(item_id) = item_id {
            self.prepare_item_access_keys(share_id, item_id, &service_account_key)
                .await?
        } else {
            self.prepare_vault_access_keys(share_id, &service_account_key)
                .await?
        };

        Ok(ServiceAccountGrantAccessRequest {
            share_id: share_id.value().to_string(),
            target_id,
            target_type,
            share_role_id: role.value(),
            keys,
            expire_time: expiration_time,
        })
    }

    async fn prepare_vault_access_keys(
        &self,
        share_id: &ShareId,
        service_account_key: &[u8],
    ) -> Result<(u8, Option<String>, Vec<KeyRotationKeyPair>)> {
        let share_keys = self
            .get_all_opened_share_keys(share_id, true)
            .await
            .context("Error getting opened share keys")?;

        let encrypted_keys: Vec<KeyRotationKeyPair> = share_keys
            .into_iter()
            .map(|k| {
                let encrypted_key =
                    crypto::encrypt(k.key(), service_account_key, EncryptionTag::VaultContent)
                        .map_err(|e| {
                            error!("Error encrypting vault key: {:?}", e);
                            anyhow!("Error encrypting vault key")
                        })?;

                Ok(KeyRotationKeyPair {
                    key: crate::utils::b64_encode(encrypted_key),
                    key_rotation: k.key_rotation,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((TargetType::Vault.value(), None, encrypted_keys))
    }

    async fn prepare_item_access_keys(
        &self,
        share_id: &ShareId,
        item_id: &ItemId,
        service_account_key: &[u8],
    ) -> Result<(u8, Option<String>, Vec<KeyRotationKeyPair>)> {
        let item_keys = self
            .get_item_keys(share_id, item_id)
            .await
            .context("Error getting item keys")?;

        let opened_item_keys = self
            .open_item_keys(share_id, item_keys)
            .await
            .context("Error opening item keys")?;

        let encrypted_keys: Vec<KeyRotationKeyPair> = opened_item_keys
            .into_iter()
            .map(|k| {
                let encrypted_key = crypto::encrypt(
                    &k.key.clone().value(),
                    service_account_key,
                    EncryptionTag::ItemKey,
                )
                .map_err(|e| {
                    error!("Error encrypting item key: {:?}", e);
                    anyhow!("Error encrypting item key")
                })?;

                Ok(KeyRotationKeyPair {
                    key: crate::utils::b64_encode(encrypted_key),
                    key_rotation: k.key_rotation,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            TargetType::Item.value(),
            Some(item_id.value().to_string()),
            encrypted_keys,
        ))
    }

    async fn send_grant_access_request(
        &self,
        service_account_id: &str,
        request: ServiceAccountGrantAccessRequest,
    ) -> Result<()> {
        let req = POST!("/pass/v1/service_account/{}/access", service_account_id)
            .body_json(&request)
            .context("Failed to create grant access request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send grant access request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_account::list::{
        ListServiceAccountsResponse, ServiceAccountData, ServiceAccountsWrapper,
    };
    use crate::test_tools::*;
    use pass_domain::PlainText;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_grant_vault_access(server: Arc<Server>) {
        const SERVICE_ACCOUNT_ID: &str = "test_sa_id";
        const SHARE_ID: &str = "test_share_id";
        const VAULT_ID: &str = "test_vault_id";
        const GRANT_PATH: &str = "/pass/v1/service_account/test_sa_id/access";
        const SHARE_KEY_PATH: &str = "/pass/v1/share/test_share_id/key";

        let client = server.pass_client().await;

        let service_account_key = crypto::generate_encryption_key();

        let user_key = client.get_primary_user_key().await.unwrap();
        let (private, public) = user_key.into_keys();
        let pgp_crypto = client.client_features.get_pgp_crypto().await;

        let encrypted_service_account_key = pgp_crypto
            .encrypt_and_sign(
                PlainText::new(service_account_key.clone()),
                public,
                private,
                None,
            )
            .await
            .expect("Error encrypting service account key");

        let encrypted_name = crypto::encrypt(
            b"TestServiceAccount",
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .expect("encryption failed");

        let encrypted_name_b64 = crate::utils::b64_encode(encrypted_name);
        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_service_account_key);

        let list_handled = server.handler("/pass/v1/service_account", move |_| {
            success(ListServiceAccountsResponse {
                service_accounts: ServiceAccountsWrapper {
                    service_accounts: vec![ServiceAccountData {
                        service_account_id: SERVICE_ACCOUNT_ID.to_string(),
                        name: encrypted_name_b64.clone(),
                        service_account_key: encrypted_key_b64.clone(),
                        expire_time: None,
                        create_time: 1704067200,
                        modify_time: 1704067200,
                    }],
                    total: 1,
                    last_token: None,
                },
            })
        });

        // Mock share keys endpoint
        let share_key_raw = crypto::generate_encryption_key();
        let encrypted_share_key = client.encrypt_for_user_key(share_key_raw.clone()).await;
        let encrypted_share_key_b64 = crate::utils::b64_encode(encrypted_share_key.clone());

        let share_key_handled = server.handler(SHARE_KEY_PATH, move |_| {
            use crate::share::keys::{GetShareKeysResponse, ShareKeyList, ShareKeyResponse};
            success(GetShareKeysResponse {
                keys: ShareKeyList {
                    keys: vec![ShareKeyResponse {
                        key_rotation: 1,
                        key: encrypted_share_key_b64.clone(),
                        create_time: 1704067200,
                    }],
                    total: 1,
                },
            })
        });

        // Mock shares list endpoint
        let share_handled = server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            use crate::share::list::{GetSharesResponse, ShareResponse};
            success(GetSharesResponse {
                shares: vec![ShareResponse {
                    share_id: SHARE_ID.to_string(),
                    address_id: TEST_ADDRESS_ID.to_string(),
                    vault_id: VAULT_ID.to_string(),
                    target_type: TargetType::Vault.value(),
                    target_id: VAULT_ID.to_string(),
                    owner: true,
                    permission: 0,
                    share_role_id: ShareRole::Owner.value(),
                    content: None,
                    content_key_rotation: None,
                    content_format_version: None,
                    expiration_time: None,
                    create_time: 1704067200,
                    group_id: None,
                }],
            })
        });

        let grant_handled = server.handler(GRANT_PATH, |_| success_code());

        let recorder = server.new_recorder();

        client
            .grant_service_account_access(
                SERVICE_ACCOUNT_ID,
                &ShareId::new(SHARE_ID.to_string()),
                None,
                &ShareRole::Viewer,
                None,
            )
            .await
            .expect("Should be able to grant vault access");

        assert_hit!(list_handled);
        assert_hit!(share_key_handled);
        assert_hit!(share_handled);
        assert_hit!(grant_handled);

        let req: ServiceAccountGrantAccessRequest = last_request!(recorder);

        assert_eq!(SHARE_ID, req.share_id);
        assert_eq!(None, req.target_id);
        assert_eq!(TargetType::Vault.value(), req.target_type);
        assert_eq!(ShareRole::Viewer.value(), req.share_role_id);
        assert!(!req.keys.is_empty());
    }
}
