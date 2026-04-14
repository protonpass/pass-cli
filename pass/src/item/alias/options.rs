/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

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
