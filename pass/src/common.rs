use anyhow::{Result, anyhow};

pub(crate) const SUCCESS_CODE: u32 = 1000;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct CodeResponse {
    #[serde(rename = "Code")]
    pub(crate) code: u32,
}

impl CodeResponse {
    pub fn is_success(&self) -> bool {
        self.code == SUCCESS_CODE
    }

    pub fn success_guard(&self) -> Result<()> {
        if !self.is_success() {
            Err(anyhow!("Invalid result code: {}", self.code))
        } else {
            Ok(())
        }
    }
}
