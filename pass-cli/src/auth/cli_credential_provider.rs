use anyhow::Result;
use pass_auth::CredentialProvider;

pub const SERVICE_ACCOUNT_ENV_VAR: &str = "PROTON_PASS_SERVICE_ACCOUNT";

pub struct CliCredentialProvider;

#[async_trait::async_trait]
impl CredentialProvider for CliCredentialProvider {
    async fn get_username(&self) -> Result<String> {
        crate::client::get_username()
    }

    async fn get_password(&self) -> Result<String> {
        crate::client::get_password()
    }

    async fn get_totp(&self) -> Result<String> {
        crate::client::get_totp()
    }

    async fn get_extra_password(&self) -> Result<String> {
        crate::client::get_extra_password()
    }

    async fn get_service_account_token(&self) -> Result<String> {
        std::env::var(SERVICE_ACCOUNT_ENV_VAR)
            .map_err(|_| anyhow::anyhow!(
                "Service account token not found. Set {SERVICE_ACCOUNT_ENV_VAR} environment variable"
            ))
    }
}
