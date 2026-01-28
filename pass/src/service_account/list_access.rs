use crate::PassClient;
use crate::pagination::SincePagination;
use anyhow::{Context, Result};
use muon::GET;
use pass_domain::{ShareId, ShareRole, TargetType};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceAccountAccess {
    pub share_id: ShareId,
    pub parent_share_id: ShareId,
    #[serde(serialize_with = "serialize_target_type")]
    pub target_type: TargetType,
    pub target_id: Option<String>,
    pub role: ShareRole,
    pub expire_time: Option<i64>,
}

fn serialize_target_type<S>(target_type: &TargetType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("{}", target_type))
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ListServiceAccountAccessResponse {
    #[serde(rename = "Access")]
    access: Vec<ServiceAccountAccessResponse>,
    #[serde(rename = "LastToken")]
    last_token: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct ServiceAccountAccessResponse {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "ParentShareID")]
    pub parent_share_id: String,
    #[serde(rename = "TargetType")]
    pub target_type: u8,
    #[serde(rename = "TargetID")]
    pub target_id: Option<String>,
    #[serde(rename = "ShareRoleID")]
    pub share_role_id: String,
    #[serde(rename = "ExpireTime")]
    pub expire_time: Option<i64>,
}

impl TryFrom<ServiceAccountAccessResponse> for ServiceAccountAccess {
    type Error = anyhow::Error;

    fn try_from(response: ServiceAccountAccessResponse) -> Result<Self> {
        Ok(Self {
            share_id: ShareId::new(response.share_id),
            parent_share_id: ShareId::new(response.parent_share_id),
            target_type: TargetType::from_value(response.target_type)
                .context("Invalid target type")?,
            target_id: response.target_id,
            role: ShareRole::from_value(&response.share_role_id, false, 0),
            expire_time: response.expire_time,
        })
    }
}

impl PassClient {
    pub async fn list_service_account_access(
        &self,
        service_account_id: &str,
    ) -> Result<Vec<ServiceAccountAccess>> {
        let access_list = self
            .fetch_service_account_access(service_account_id)
            .await
            .context("Error fetching service account access")?;

        let mut result = Vec::with_capacity(access_list.len());
        for access in access_list {
            result.push(ServiceAccountAccess::try_from(access).context(
                "Error converting ServiceAccountAccessResponse to ServiceAccountAccess",
            )?);
        }

        Ok(result)
    }

    async fn fetch_service_account_access(
        &self,
        service_account_id: &str,
    ) -> Result<Vec<ServiceAccountAccessResponse>> {
        let mut access_list = vec![];
        let mut pagination = SincePagination::default();

        loop {
            let mut req = GET!("/pass/v1/service_account/{}/access", service_account_id);
            if let Some(ref since) = pagination.since {
                req = req.query(("Since", since));
            }

            let res = self
                .send(req)
                .await
                .context("Error fetching service account access")?;
            let response: ListServiceAccountAccessResponse = assert_response!(res);

            let should_break = response.access.len() < pagination.page_size;
            access_list.extend(response.access);

            if should_break {
                break;
            }

            pagination = match pagination.next(response.last_token) {
                Some(p) => p,
                None => break,
            };
        }

        Ok(access_list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_list_service_account_access(server: Arc<Server>) {
        const SERVICE_ACCOUNT_ID: &str = "test_sa_id";
        const SHARE_ID_1: &str = "share_1";
        const PARENT_SHARE_ID_1: &str = "parent_share_1";
        const SHARE_ID_2: &str = "share_2";
        const PARENT_SHARE_ID_2: &str = "parent_share_2";
        const LIST_ACCESS_PATH: &str = "/pass/v1/service_account/test_sa_id/access";

        let client = server.pass_client().await;

        let list_access_handled = server.handler(LIST_ACCESS_PATH, move |_| {
            success(ListServiceAccountAccessResponse {
                access: vec![
                    ServiceAccountAccessResponse {
                        share_id: SHARE_ID_1.to_string(),
                        target_type: TargetType::Vault.value(),
                        target_id: None,
                        share_role_id: ShareRole::Viewer.value(),
                        expire_time: None,
                        parent_share_id: PARENT_SHARE_ID_1.to_string(),
                    },
                    ServiceAccountAccessResponse {
                        share_id: SHARE_ID_2.to_string(),
                        target_type: TargetType::Item.value(),
                        target_id: Some("item_123".to_string()),
                        share_role_id: ShareRole::Editor.value(),
                        expire_time: Some(1735689600),
                        parent_share_id: PARENT_SHARE_ID_2.to_string(),
                    },
                ],
                last_token: None,
            })
        });

        let access_list = client
            .list_service_account_access(SERVICE_ACCOUNT_ID)
            .await
            .expect("Should be able to list service account access");

        assert_hit!(list_access_handled);
        assert_eq!(2, access_list.len());

        assert_eq!(SHARE_ID_1, access_list[0].share_id.value());
        assert_eq!(PARENT_SHARE_ID_1, access_list[0].parent_share_id.value());
        assert!(matches!(access_list[0].target_type, TargetType::Vault));
        assert_eq!(None, access_list[0].target_id);
        assert_eq!(ShareRole::Viewer, access_list[0].role);

        assert_eq!(SHARE_ID_2, access_list[1].share_id.value());
        assert_eq!(PARENT_SHARE_ID_2, access_list[1].parent_share_id.value());
        assert!(matches!(access_list[1].target_type, TargetType::Item));
        assert_eq!(Some("item_123".to_string()), access_list[1].target_id);
        assert_eq!(ShareRole::Editor, access_list[1].role);
    }
}
