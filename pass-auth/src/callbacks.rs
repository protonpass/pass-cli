use anyhow::Result;

#[async_trait::async_trait]
pub trait AuthEventHandler: Send + Sync {
    async fn on_web_login_url_generated(&self, url: &str) -> Result<()>;
    async fn on_poll_progress(&self, attempt: u32, max_attempts: u32) -> Result<()>;
    async fn on_auth_success(&self, message: &str) -> Result<()>;
    async fn on_extra_password_required(&self) -> Result<()>;
    async fn on_info(&self, message: &str) -> Result<()>;
    async fn on_warning(&self, message: &str) -> Result<()>;
    async fn on_error(&self, message: &str) -> Result<()>;
}

#[async_trait::async_trait]
pub trait CredentialProvider: Send + Sync {
    async fn get_username(&self) -> Result<String>;
    async fn get_password(&self) -> Result<String>;
    async fn get_totp(&self) -> Result<String>;
    async fn get_extra_password(&self) -> Result<String>;
    async fn get_service_account_token(&self) -> Result<String>;
}
