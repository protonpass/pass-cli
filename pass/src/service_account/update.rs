use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use muon::PUT;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ServiceAccountId, crypto};

#[derive(Debug)]
pub struct UpdateServiceAccountArgs {
    name: String,
}

impl UpdateServiceAccountArgs {
    pub fn new(name: String) -> Result<Self> {
        if name.trim().is_empty() {
            return Err(anyhow!("Empty service account name"));
        }

        Ok(Self { name })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct UpdateServiceAccountRequest {
    #[serde(rename = "Name")]
    pub name: String,
}

impl PassClient {
    pub async fn update_service_account(
        &self,
        service_account_id: &ServiceAccountId,
        args: UpdateServiceAccountArgs,
    ) -> Result<()> {
        info!(
            "Updating service account: {} to name: {}",
            service_account_id, args.name
        );

        let service_account_key = self
            .get_service_account_key(service_account_id)
            .await
            .context("Failed to get service account key")?;

        let encrypted_name = crypto::encrypt(
            args.name.as_bytes(),
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .map_err(|e| {
            error!("Error encrypting service account name: {:?}", e);
            anyhow!("Error encrypting service account name")
        })?;

        let req_body = UpdateServiceAccountRequest {
            name: crate::utils::b64_encode(encrypted_name),
        };

        let req = PUT!("/pass/v1/service_account/{}", service_account_id)
            .body_json(&req_body)
            .context("Failed to create update service account request")?;

        let res = self
            .send(req)
            .await
            .context("Failed to send update service account request")?;

        let _: crate::common::CodeResponse = assert_response!(res);

        info!(
            "Service account updated successfully: ID={}",
            service_account_id
        );

        Ok(())
    }

    pub(crate) async fn get_service_account_key(
        &self,
        service_account_id: &ServiceAccountId,
    ) -> Result<Vec<u8>> {
        let service_accounts = self
            .list_service_accounts()
            .await
            .context("Failed to list service accounts")?;

        let service_account = service_accounts
            .iter()
            .find(|sa| service_account_id.eq(&sa.service_account_id))
            .ok_or_else(|| anyhow!("Service account not found: {}", service_account_id))?;

        service_account
            .service_account_key
            .clone()
            .ok_or_else(|| anyhow!("Service account key not available"))
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
    async fn test_update_service_account(server: Arc<Server>) {
        const OLD_NAME: &str = "OldServiceAccountName";
        const NEW_NAME: &str = "NewServiceAccountName";
        const SERVICE_ACCOUNT_ID: &str = "test_sa_id";
        const UPDATE_PATH: &str = "/pass/v1/service_account/test_sa_id";

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

        let encrypted_old_name = crypto::encrypt(
            OLD_NAME.as_bytes(),
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .expect("encryption failed");

        let encrypted_old_name_b64 = crate::utils::b64_encode(encrypted_old_name);
        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_service_account_key);

        let list_handled = server.handler("/pass/v1/service_account", move |_| {
            success(ListServiceAccountsResponse {
                service_accounts: ServiceAccountsWrapper {
                    service_accounts: vec![ServiceAccountData {
                        service_account_id: SERVICE_ACCOUNT_ID.to_string(),
                        name: encrypted_old_name_b64.clone(),
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

        let update_handled = server.handler(UPDATE_PATH, |_| success_code());

        let recorder = server.new_recorder();

        client
            .update_service_account(
                &ServiceAccountId::new(SERVICE_ACCOUNT_ID.to_string()),
                UpdateServiceAccountArgs::new(NEW_NAME.to_string()).unwrap(),
            )
            .await
            .expect("Should be able to update service account");

        assert_hit!(list_handled);
        assert_hit!(update_handled);

        let req: UpdateServiceAccountRequest = last_request!(recorder);

        let encrypted_name_bytes = crate::utils::b64_decode(&req.name).unwrap();
        let decrypted_name = crypto::decrypt(
            &encrypted_name_bytes,
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .expect("Error decrypting service account name");

        let parsed_name = String::from_utf8(decrypted_name).expect("Invalid UTF-8");
        assert_eq!(NEW_NAME, parsed_name);
    }

    #[test]
    fn test_empty_name_validation() {
        let result = UpdateServiceAccountArgs::new("".to_string());
        assert!(result.is_err());
        assert_eq!(
            "Empty service account name",
            result.unwrap_err().to_string()
        );

        let result = UpdateServiceAccountArgs::new("   ".to_string());
        assert!(result.is_err());
    }
}
