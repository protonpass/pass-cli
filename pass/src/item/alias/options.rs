use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::ShareId;

#[derive(serde::Deserialize)]
pub(crate) struct AliasMailbox {
    #[serde(rename = "ID")]
    pub id: i64,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub(crate) struct AliasSuffix {
    #[serde(rename = "Suffix")]
    pub suffix: String,
    #[serde(rename = "SignedSuffix")]
    pub signed_suffix: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct AliasOptionsResponse {
    #[serde(rename = "Suffixes")]
    pub suffixes: Vec<AliasSuffix>,
    #[serde(rename = "Mailboxes")]
    pub mailboxes: Vec<AliasMailbox>,
    #[serde(rename = "CanCreateAlias")]
    pub can_create_alias: bool,
}

#[derive(serde::Deserialize)]
pub(crate) struct GetAliasOptionsResponse {
    #[serde(rename = "Options")]
    pub options: AliasOptionsResponse,
}

impl<C: PassClientContext> PassClient<C> {
    pub(crate) async fn get_alias_options(
        &self,
        share_id: &ShareId,
    ) -> Result<AliasOptionsResponse> {
        let res = self
            .send(GET!("/pass/v1/share/{share_id}/alias/options"))
            .await
            .context("Error fetching alias options")?;

        let res: GetAliasOptionsResponse = assert_response!(res);

        if !res.options.can_create_alias {
            return Err(anyhow!(
                "Server does not support creating aliases for this share"
            ));
        }

        Ok(res.options)
    }
}
