use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use base64::Engine;
use muon::POST;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{PlainText, ServiceAccountId, crypto};

#[derive(Debug)]
pub struct CreateServiceAccountArgs {
    name: String,
    expiration_time: Option<i64>,
}

impl CreateServiceAccountArgs {
    pub fn new(name: String, expiration_time: Option<i64>) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(anyhow!("Empty service account name"));
        }

        Ok(Self {
            name,
            expiration_time,
        })
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreateServiceAccountRequest {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ServiceAccountKey")]
    pub service_account_key: String,
    #[serde(rename = "ExpireTime", skip_serializing_if = "Option::is_none")]
    pub expire_time: Option<i64>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct CreateServiceAccountResponseData {
    #[serde(rename = "ServiceAccount")]
    pub service_account: ServiceAccountData,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct ServiceAccountData {
    #[serde(rename = "ServiceAccountID")]
    pub service_account_id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ServiceAccountKey")]
    pub service_account_key: String,
    #[serde(rename = "ExpireTime")]
    pub expire_time: Option<i64>,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "ModifyTime")]
    pub modify_time: i64,
    #[serde(rename = "Token")]
    pub token: String,
}

pub struct CreateServiceAccountResponse {
    pub service_account_id: ServiceAccountId,
    pub name: String,
    pub service_account_key: String,
    pub expire_time: Option<i64>,
    pub create_time: i64,
    pub modify_time: i64,
    pub token: String,
    pub raw_service_account_key: Vec<u8>,
    pub env_var: String,
}

impl PassClient {
    pub async fn create_service_account(
        &self,
        args: CreateServiceAccountArgs,
    ) -> Result<CreateServiceAccountResponse> {
        info!("Creating service account: {}", args.name);

        let (req, raw_service_account_key) = self
            .create_service_account_request(args)
            .await
            .context("Failed to create service account request")?;

        let req = POST!("/pass/v1/service_account")
            .body_json(&req)
            .context("Failed to create service account request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send create service account request")?;

        let response: CreateServiceAccountResponseData = assert_response!(res);

        let sa = response.service_account;
        info!(
            "Service account created successfully: ID={}",
            sa.service_account_id
        );

        let service_account_key_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&raw_service_account_key);

        let env_var = format!("{}::{}", sa.token, service_account_key_b64);
        Ok(CreateServiceAccountResponse {
            service_account_id: ServiceAccountId::new(sa.service_account_id),
            name: sa.name,
            service_account_key: sa.service_account_key,
            expire_time: sa.expire_time,
            create_time: sa.create_time,
            modify_time: sa.modify_time,
            token: sa.token,
            env_var,
            raw_service_account_key,
        })
    }

    async fn create_service_account_request(
        &self,
        args: CreateServiceAccountArgs,
    ) -> Result<(CreateServiceAccountRequest, Vec<u8>)> {
        let service_account_key = crypto::generate_encryption_key();

        let encrypted_name = crypto::encrypt(
            args.name.as_bytes(),
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .map_err(|e| {
            error!("Error encrypting service account name: {:?}", e);
            anyhow!("Error encrypting service account name")
        })?;

        let user_key = self
            .get_primary_user_key()
            .await
            .context("Error getting primary user key")?;
        let (private, public) = user_key.into_keys();
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let encrypted_service_account_key = pgp_crypto
            .encrypt_and_sign(
                PlainText::new(service_account_key.clone()),
                public,
                private,
                None,
            )
            .await
            .context("Error encrypting and signing service account key")?;

        Ok((
            CreateServiceAccountRequest {
                name: crate::utils::b64_encode(encrypted_name),
                service_account_key: crate::utils::b64_encode(encrypted_service_account_key),
                expire_time: args.expiration_time,
            },
            service_account_key,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_create_service_account(server: Arc<Server>) {
        const SERVICE_ACCOUNT_NAME: &str = "MyTestServiceAccount";
        const SERVICE_ACCOUNT_ID: &str = "MyServiceAccountID";
        const TOKEN: &str = "ppsa_test_token_123";
        const CREATE_TIME: i64 = 1704067200;
        const MODIFY_TIME: i64 = 1704067200;

        let client = server.pass_client().await;
        let handled = server.handler("/pass/v1/service_account", |_| {
            success(CreateServiceAccountResponseData {
                service_account: ServiceAccountData {
                    service_account_id: SERVICE_ACCOUNT_ID.to_string(),
                    name: "encrypted_name".to_string(),
                    service_account_key: "encrypted_key".to_string(),
                    expire_time: None,
                    create_time: CREATE_TIME,
                    modify_time: MODIFY_TIME,
                    token: TOKEN.to_string(),
                },
            })
        });

        let recorder = server.new_recorder();
        let response = client
            .create_service_account(
                CreateServiceAccountArgs::new(SERVICE_ACCOUNT_NAME.to_string(), None).unwrap(),
            )
            .await
            .expect("Should be able to create the service account");

        assert_eq!(SERVICE_ACCOUNT_ID, response.service_account_id.value());
        assert_eq!(TOKEN, response.token);
        assert_eq!(32, response.raw_service_account_key.len());

        assert_hit!(handled);

        let req: CreateServiceAccountRequest = last_request!(recorder);

        let user_key = client.get_primary_user_key().await.unwrap();
        let (private, public) = user_key.into_keys();

        let encrypted_service_account_key =
            crate::utils::b64_decode(&req.service_account_key).unwrap();
        let pgp_crypto = client.client_features.get_pgp_crypto().await;
        let decrypted_service_account_key = pgp_crypto
            .decrypt_and_verify(
                encrypted_service_account_key,
                vec![private],
                vec![public],
                None,
            )
            .await
            .expect("Error decrypting and verifying service account key");
        assert_eq!(32, decrypted_service_account_key.len());
        assert_eq!(
            response.raw_service_account_key,
            decrypted_service_account_key
        );

        let encrypted_name = crate::utils::b64_decode(&req.name).unwrap();
        let decrypted_name = crypto::decrypt(
            &encrypted_name,
            &decrypted_service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .expect("Error decrypting service account name");

        let parsed_name = String::from_utf8(decrypted_name).expect("Invalid UTF-8");
        assert_eq!(SERVICE_ACCOUNT_NAME, parsed_name);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_create_service_account_with_expiration(server: Arc<Server>) {
        const SERVICE_ACCOUNT_NAME: &str = "ExpiringServiceAccount";
        const EXPIRATION_TIME: i64 = 1735689600;

        let client = server.pass_client().await;
        let handled = server.handler("/pass/v1/service_account", |_| {
            success(CreateServiceAccountResponseData {
                service_account: ServiceAccountData {
                    service_account_id: "test_id".to_string(),
                    name: "encrypted_name".to_string(),
                    service_account_key: "encrypted_key".to_string(),
                    expire_time: Some(EXPIRATION_TIME),
                    create_time: 1704067200,
                    modify_time: 1704067200,
                    token: "ppsa_token".to_string(),
                },
            })
        });

        let recorder = server.new_recorder();
        let response = client
            .create_service_account(
                CreateServiceAccountArgs::new(
                    SERVICE_ACCOUNT_NAME.to_string(),
                    Some(EXPIRATION_TIME),
                )
                .unwrap(),
            )
            .await
            .expect("Should be able to create the service account with expiration");

        assert_eq!(Some(EXPIRATION_TIME), response.expire_time);
        assert_hit!(handled);

        let req: CreateServiceAccountRequest = last_request!(recorder);
        assert_eq!(Some(EXPIRATION_TIME), req.expire_time);
    }

    #[test]
    fn test_empty_name_validation() {
        let result = CreateServiceAccountArgs::new("".to_string(), None);
        assert!(result.is_err());
        assert_eq!(
            "Empty service account name",
            result.unwrap_err().to_string()
        );

        let result = CreateServiceAccountArgs::new("   ".to_string(), None);
        assert!(result.is_err());
    }
}
