use crate::{PassClient, PassClientContext};
use anyhow::Result;
use muon::GET;
use muon::env::Environment;

#[derive(Debug)]
pub struct UserInfo {
    pub user: UserInfoUser,
    pub env: Environment,
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

impl<C: PassClientContext> PassClient<C> {
    pub async fn get_info(&self) -> Result<UserInfo> {
        let res = self.send(GET!("/core/v4/users")).await?;
        let response: GetUserResponse = assert_response!(res);
        Ok(UserInfo {
            user: UserInfoUser::from(response.user),
            env: self.client.env().clone(),
        })
    }

    pub async fn get_personal_access_token_name(&self) -> Result<String> {
        let personal_access_token_data = self.get_personal_access_token_self().await?;
        Ok(personal_access_token_data.name)
    }

    async fn get_personal_access_token_self(&self) -> Result<PersonalAccessTokenSelfData> {
        let res = self
            .send(GET!("/account/v4/personal-access-token/self"))
            .await?;
        let response: PersonalAccessTokenSelfResponse = assert_response!(res);
        Ok(response.personal_access_token)
    }
}

#[derive(Debug, serde::Deserialize)]
struct PersonalAccessTokenSelfResponse {
    #[serde(rename = "PersonalAccessToken")]
    personal_access_token: PersonalAccessTokenSelfData,
}

#[derive(Debug, serde::Deserialize)]
struct PersonalAccessTokenSelfData {
    #[serde(rename = "PersonalAccessTokenID")]
    #[allow(dead_code)]
    pub personal_access_token_id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ExpireTime")]
    #[allow(dead_code)]
    pub expire_time: Option<i64>,
}
