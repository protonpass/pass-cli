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

use crate::common::CodeResponse;
use crate::{PassClient, PassClientContext};
use anyhow::{Context, Result};
use muon::DELETE;
use pass_domain::{FolderId, ShareId};

#[derive(Clone, Debug, serde::Serialize)]
struct DeleteFolderPayload {
    #[serde(rename = "FolderIDs")]
    folder_ids: Vec<String>,
}

impl<C: PassClientContext> PassClient<C> {
    pub async fn delete_folder(&self, share_id: &ShareId, folder_id: &FolderId) -> Result<()> {
        let payload = DeleteFolderPayload {
            folder_ids: vec![folder_id.to_string()],
        };
        let req = DELETE!("/pass/v1/share/{share_id}/folder")
            .body_json(payload)
            .context("Error creating delete folder request")?;

        let res = self
            .send(req)
            .await
            .context("Error sending delete folder request")?;

        let response: CodeResponse = assert_response!(res);
        response.success_guard()?;

        // Clear folders cache since we deleted a folder
        self.clear_folders_cache(share_id).await;

        debug!("Folder {} deleted", folder_id);

        Ok(())
    }
}
