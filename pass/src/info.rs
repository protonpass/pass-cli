use crate::PassClient;
use anyhow::{Result, anyhow};
use muon::GET;
use muon::env::EnvId;
use muon::rest::core::v4::users::User;

pub struct UserInfo {
    pub user: User,
    pub env: EnvId,
}

impl PassClient {
    pub async fn get_info(&self) -> Result<UserInfo> {
        let res = self.send(GET!("/core/v4/users")).await?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }

        let res: muon::rest::core::v4::users::GetRes = res.ok()?.into_body_json()?;
        Ok(UserInfo {
            user: res.user,
            env: self.client.env().clone(),
        })
    }
}
