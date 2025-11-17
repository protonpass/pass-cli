use crate::PassClient;
use anyhow::Result;
use muon::GET;

#[derive(Debug)]
pub struct AccountUserSettings {
    pub telemetry_enabled: bool,
}

#[derive(Debug, serde::Deserialize)]
struct AccountUserSettingsResponse {
    #[serde(rename = "Telemetry")]
    pub telemetry: u8,
}

impl PassClient {
    pub async fn get_account_user_settings(&self) -> Result<AccountUserSettings> {
        let res = self.send(GET!("/core/v4/settings")).await?;
        let response: AccountUserSettingsResponse = assert_response!(res);

        Ok(AccountUserSettings {
            telemetry_enabled: response.telemetry == 1,
        })
    }
}
