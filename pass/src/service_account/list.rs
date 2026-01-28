use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::crypto::EncryptionTag;
use pass_domain::{ServiceAccountId, crypto};

const PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ServiceAccountData {
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
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct ServiceAccountsWrapper {
    #[serde(rename = "ServiceAccounts")]
    pub(crate) service_accounts: Vec<ServiceAccountData>,
    #[serde(rename = "Total")]
    #[allow(dead_code)]
    pub(crate) total: i64,
    #[serde(rename = "LastToken")]
    pub(crate) last_token: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(Clone))]
pub(crate) struct ListServiceAccountsResponse {
    #[serde(rename = "ServiceAccounts")]
    pub(crate) service_accounts: ServiceAccountsWrapper,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ServiceAccount {
    pub service_account_id: ServiceAccountId,
    pub name: String,
    pub expire_time: Option<i64>,
    #[serde(skip)]
    pub(crate) service_account_key: Option<Vec<u8>>,
}

impl PassClient {
    pub async fn list_service_accounts(&self) -> Result<Vec<ServiceAccount>> {
        info!("Fetching service accounts");

        let mut all_service_accounts = Vec::new();
        let mut last_token: Option<String> = None;

        loop {
            let mut req =
                GET!("/pass/v1/service_account").query(("PageSize", format!("{}", PAGE_SIZE)));

            if let Some(token) = &last_token {
                req = req.query(("Since", token.clone()));
            }

            let res = self
                .send(req)
                .await
                .context("Error sending list service accounts request")?;

            let response: ListServiceAccountsResponse = assert_response!(res);

            let wrapper = response.service_accounts;

            for sa_data in wrapper.service_accounts {
                match self.open_service_account(&sa_data).await {
                    Ok(sa) => all_service_accounts.push(sa),
                    Err(e) => {
                        warn!(
                            "Error opening service account {}: {}. Skipping.",
                            sa_data.service_account_id, e
                        );
                    }
                }
            }

            match wrapper.last_token {
                Some(token) if !token.is_empty() => {
                    last_token = Some(token);
                }
                _ => break,
            }
        }

        info!(
            "Successfully fetched {} service accounts",
            all_service_accounts.len()
        );

        Ok(all_service_accounts)
    }

    async fn open_service_account(&self, sa_data: &ServiceAccountData) -> Result<ServiceAccount> {
        let encrypted_service_account_key = crate::utils::b64_decode(&sa_data.service_account_key)
            .context("Error decoding service account key")?;

        let user_key = self
            .get_primary_user_key()
            .await
            .context("Error getting primary user key")?;
        let (private, public) = user_key.into_keys();
        let pgp_crypto = self.client_features.get_pgp_crypto().await;

        let decrypted_service_account_key = pgp_crypto
            .decrypt_and_verify(
                encrypted_service_account_key,
                vec![private],
                vec![public],
                None,
            )
            .await
            .context("Error decrypting and verifying service account key")?;

        let encrypted_name = crate::utils::b64_decode(&sa_data.name)
            .context("Error decoding service account name")?;

        let decrypted_name = crypto::decrypt(
            &encrypted_name,
            &decrypted_service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .map_err(|e| {
            error!("Error decrypting service account name: {e}");
            anyhow!("Error decrypting service account name")
        })?;

        let name =
            String::from_utf8(decrypted_name).context("Service account name is not valid UTF-8")?;

        Ok(ServiceAccount {
            service_account_id: ServiceAccountId::new(sa_data.service_account_id.clone()),
            name,
            expire_time: sa_data.expire_time,
            service_account_key: Some(decrypted_service_account_key),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use pass_domain::PlainText;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_list_service_accounts_empty(server: Arc<Server>) {
        let client = server.pass_client().await;
        let handled = server.handler("/pass/v1/service_account", |_| {
            success(ListServiceAccountsResponse {
                service_accounts: ServiceAccountsWrapper {
                    service_accounts: vec![],
                    total: 0,
                    last_token: None,
                },
            })
        });

        let result = client
            .list_service_accounts()
            .await
            .expect("Should be able to list service accounts");

        assert_eq!(0, result.len());
        assert_hit!(handled);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_service_accounts_single(server: Arc<Server>) {
        const SERVICE_ACCOUNT_NAME: &str = "MyTestServiceAccount";
        const SERVICE_ACCOUNT_ID: &str = "test_id_123";
        const CREATE_TIME: i64 = 1704067200;
        const MODIFY_TIME: i64 = 1704067200;

        let client = server.pass_client().await;

        let service_account_key = crypto::generate_encryption_key();
        let encrypted_name = crypto::encrypt(
            SERVICE_ACCOUNT_NAME.as_bytes(),
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .expect("encryption failed");

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

        let encrypted_name_b64 = crate::utils::b64_encode(encrypted_name);
        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_service_account_key);

        let handled = server.handler("/pass/v1/service_account", move |_| {
            success(ListServiceAccountsResponse {
                service_accounts: ServiceAccountsWrapper {
                    service_accounts: vec![ServiceAccountData {
                        service_account_id: SERVICE_ACCOUNT_ID.to_string(),
                        name: encrypted_name_b64.clone(),
                        service_account_key: encrypted_key_b64.clone(),
                        expire_time: None,
                        create_time: CREATE_TIME,
                        modify_time: MODIFY_TIME,
                    }],
                    total: 1,
                    last_token: None,
                },
            })
        });

        let result = client
            .list_service_accounts()
            .await
            .expect("Should be able to list service accounts");

        assert_eq!(1, result.len());
        assert_eq!(SERVICE_ACCOUNT_ID, result[0].service_account_id.value());
        assert_eq!(SERVICE_ACCOUNT_NAME, result[0].name);
        assert_eq!(None, result[0].expire_time);

        assert_hit!(handled);
    }

    #[muon::test(scheme(HTTP))]
    async fn test_list_service_accounts_with_expiration(server: Arc<Server>) {
        const SERVICE_ACCOUNT_NAME: &str = "ExpiringAccount";
        const SERVICE_ACCOUNT_ID: &str = "expiring_id";
        const EXPIRATION_TIME: i64 = 1735689600;

        let client = server.pass_client().await;

        let service_account_key = crypto::generate_encryption_key();
        let encrypted_name = crypto::encrypt(
            SERVICE_ACCOUNT_NAME.as_bytes(),
            &service_account_key,
            EncryptionTag::ServiceAccountName,
        )
        .expect("encryption failed");

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

        let encrypted_name_b64 = crate::utils::b64_encode(encrypted_name);
        let encrypted_key_b64 = crate::utils::b64_encode(encrypted_service_account_key);

        let handled = server.handler("/pass/v1/service_account", move |_| {
            success(ListServiceAccountsResponse {
                service_accounts: ServiceAccountsWrapper {
                    service_accounts: vec![ServiceAccountData {
                        service_account_id: SERVICE_ACCOUNT_ID.to_string(),
                        name: encrypted_name_b64.clone(),
                        service_account_key: encrypted_key_b64.clone(),
                        expire_time: Some(EXPIRATION_TIME),
                        create_time: 1704067200,
                        modify_time: 1704067200,
                    }],
                    total: 1,
                    last_token: None,
                },
            })
        });

        let result = client
            .list_service_accounts()
            .await
            .expect("Should be able to list service accounts");

        assert_eq!(1, result.len());
        assert_eq!(Some(EXPIRATION_TIME), result[0].expire_time);

        assert_hit!(handled);
    }
}
