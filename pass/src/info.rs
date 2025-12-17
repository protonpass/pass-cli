use crate::PassClient;
use anyhow::Result;
use muon::GET;
use muon::env::EnvId;

#[derive(Debug)]
pub struct UserInfo {
    pub user: UserInfoUser,
    pub env: EnvId,
}

#[derive(Debug)]
pub struct UserInfoUser {
    pub id: String,
    pub name: String,
    pub email: String,
}

impl From<UserResponse> for UserInfoUser {
    fn from(value: UserResponse) -> Self {
        Self {
            id: value.id,
            name: value.name.unwrap_or_else(|| value.email.clone()),
            email: value.email,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct GetUserResponse {
    #[serde(rename = "User")]
    user: UserResponse,
}

#[derive(Debug, serde::Deserialize)]
struct UserResponse {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "Email")]
    pub email: String,
}

impl PassClient {
    pub async fn get_info(&self) -> Result<UserInfo> {
        let res = self.send(GET!("/core/v4/users")).await?;
        let response: GetUserResponse = assert_response!(res);
        Ok(UserInfo {
            user: UserInfoUser::from(response.user),
            env: self.client.env().clone(),
        })
    }
}
