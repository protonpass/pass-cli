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
