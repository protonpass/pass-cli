use crate::PassClient;
use anyhow::{Context, Result, anyhow};
use muon::GET;
use pass_domain::{
    AddressId, GroupId, ItemId, Permission, Share, ShareContent, ShareId, ShareRole, ShareType,
    VaultId,
};

const TARGET_TYPE_VAULT: u8 = 1;
const TARGET_TYPE_ITEM: u8 = 2;

struct GetSharesCacheType;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct GetSharesResponse {
    #[serde(rename = "Shares")]
    pub shares: Vec<ShareResponse>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct ShareResponse {
    #[serde(rename = "ShareID")]
    pub share_id: String,
    #[serde(rename = "AddressID")]
    pub address_id: String,
    #[serde(rename = "VaultID")]
    pub vault_id: String,
    #[serde(rename = "TargetType")]
    pub target_type: u8,
    #[serde(rename = "TargetID")]
    pub target_id: String,
    #[serde(rename = "Owner")]
    pub owner: bool,
    #[serde(rename = "Permission")]
    pub permission: u16,
    #[serde(rename = "ShareRoleID")]
    pub share_role_id: String,
    #[serde(rename = "Content")]
    pub content: Option<String>,
    #[serde(rename = "ContentKeyRotation")]
    pub content_key_rotation: Option<u8>,
    #[serde(rename = "ContentFormatVersion")]
    pub content_format_version: Option<u8>,
    #[serde(rename = "ExpireTime")]
    pub expiration_time: Option<i64>,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "GroupID")]
    pub group_id: Option<String>,
}

impl TryFrom<ShareResponse> for Share {
    type Error = anyhow::Error;
    fn try_from(value: ShareResponse) -> Result<Self> {
        let share_content = match (
            value.content,
            value.content_format_version,
            value.content_key_rotation,
        ) {
            (Some(content), Some(cfv), Some(ckr)) => Some(ShareContent {
                content: crate::utils::b64_decode(&content)
                    .context("Error decoding share content")?,
                share_key_rotation: ckr,
                content_format_version: cfv,
            }),
            _ => None,
        };

        Ok(Self {
            id: ShareId::new(value.share_id),
            address_id: AddressId::new(value.address_id),
            vault_id: VaultId::new(value.vault_id.clone()),
            permission: Permission::new_from_role(
                &value.share_role_id,
                value.owner,
                value.permission,
            ),
            share_role: ShareRole::from_value(&value.share_role_id, value.owner, value.permission),
            content: share_content,
            group_id: value.group_id.map(GroupId::new),
            share_type: match value.target_type {
                TARGET_TYPE_VAULT => ShareType::Vault {
                    vault_id: VaultId::new(value.target_id),
                },
                TARGET_TYPE_ITEM => ShareType::Item {
                    vault_id: VaultId::new(value.vault_id),
                    item_id: ItemId::new(value.target_id),
                },
                _ => anyhow::bail!("Invalid share type {}", value.target_type),
            },
        })
    }
}

impl PassClient {
    pub async fn list_shares(&self) -> Result<Vec<Share>> {
        match self.get_cached_shares().await {
            Some(s) => Ok(s),
            None => self.fetch_shares().await,
        }
    }

    async fn fetch_shares(&self) -> Result<Vec<Share>> {
        let res = self.client.send(GET!("/pass/v1/share")).await?;
        if !res.status().is_success() {
            return Err(anyhow!("HTTP Status: {:?}", res.status()));
        }

        let res: GetSharesResponse = res.body_json()?;
        let mut result = vec![];
        for share in res.shares {
            result.push(share.try_into()?);
        }

        self.cache.store(GetSharesCacheType, result.clone()).await;
        Ok(result)
    }

    pub async fn get_share(&self, share_id: &ShareId) -> Result<Share> {
        let (shares, fetched) = match self.get_cached_shares().await {
            Some(cache) => (cache, false),
            None => match self.fetch_shares().await {
                Ok(shares) => (shares, true),
                Err(e) => return Err(anyhow::anyhow!("Error fetching shares: {}", e)),
            },
        };

        for share in shares {
            if share.id.eq(share_id) {
                return Ok(share);
            }
        }

        if fetched {
            return Err(anyhow::anyhow!("Share with id {} not found", share_id));
        }

        let shares = self.fetch_shares().await.context("Error fetching shares")?;

        for share in shares {
            if share.id.eq(share_id) {
                return Ok(share);
            }
        }

        Err(anyhow::anyhow!("Share with id {} not found", share_id))
    }

    async fn get_cached_shares(&self) -> Option<Vec<Share>> {
        self.cache.get(GetSharesCacheType).await
    }

    pub(crate) async fn clear_shares_cache(&self) {
        self.cache.delete(GetSharesCacheType).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use muon::test::server::{HTTP, Server};
    use pass_domain::TargetType;

    #[muon::test(scheme(HTTP))]
    async fn test_fetch_shares_empty_list(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse { shares: vec![] })
        });

        let recorder = server.new_recorder();
        let shares = client
            .list_shares()
            .await
            .expect("Should be able to list shares");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        assert!(shares.is_empty());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_fetch_shares_caches_result(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse { shares: vec![] })
        });

        let recorder = server.new_recorder();

        // First fetch
        client
            .list_shares()
            .await
            .expect("Should be able to list shares");

        // Second fetch
        client
            .list_shares()
            .await
            .expect("Should be able to list shares");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_fetch_shares_cache_used_for_get_share(server: Arc<Server>) {
        const SHARE_ID: &str = "Share1ID";

        let client = server.pass_client().await;

        server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse {
                shares: vec![ShareResponse {
                    share_id: SHARE_ID.to_string(),
                    address_id: "".to_string(),
                    vault_id: "".to_string(),
                    target_type: TargetType::Vault.value(),
                    target_id: "".to_string(),
                    owner: false,
                    permission: 0,
                    share_role_id: "1".to_string(),
                    content: None,
                    content_key_rotation: None,
                    content_format_version: None,
                    expiration_time: None,
                    create_time: 0,
                    group_id: None,
                }],
            })
        });

        let recorder = server.new_recorder();

        // All shares fetch
        client
            .list_shares()
            .await
            .expect("Should be able to list shares");

        let requests_1 = recorder.read().len();

        // Single share fetch
        let share = client
            .get_share(&share_id!(SHARE_ID))
            .await
            .expect("Should be able to get share");

        let requests_2 = recorder.read().len();
        assert_eq!(requests_1, requests_2);

        assert_eq!(SHARE_ID, share.id.value());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_fetch_shares_processes_response(server: Arc<Server>) {
        const SHARE_1_ID: &str = "Share1ID";
        const SHARE_1_ADDRESS_ID: &str = "Share1AddressID";
        const SHARE_1_VAULT_ID: &str = "Share1VaultID";
        const SHARE_2_ID: &str = "Share2ID";
        const SHARE_2_ADDRESS_ID: &str = "Share2AddressID";
        const SHARE_2_VAULT_ID: &str = "Share2VaultID";
        const SHARE_2_ITEM_ID: &str = "Share2ItemID";

        let client = server.pass_client().await;

        let content_1 = crate::utils::b64_encode(random_string(10).as_bytes());
        let share_response_1 = ShareResponse {
            share_id: SHARE_1_ID.to_string(),
            address_id: SHARE_1_ADDRESS_ID.to_string(),
            vault_id: SHARE_1_VAULT_ID.to_string(),
            target_type: TargetType::Vault.value(),
            target_id: SHARE_1_VAULT_ID.to_string(),
            owner: true,
            permission: 0,
            share_role_id: "1".to_string(),
            content: Some(content_1.clone()),
            content_key_rotation: Some(1),
            content_format_version: Some(1),
            expiration_time: None,
            create_time: 123456789,
            group_id: None,
        };
        let share_response_1_clone = share_response_1.clone();

        let share_response_2 = ShareResponse {
            share_id: SHARE_2_ID.to_string(),
            address_id: SHARE_2_ADDRESS_ID.to_string(),
            vault_id: SHARE_2_VAULT_ID.to_string(),
            target_type: TargetType::Item.value(),
            target_id: SHARE_2_ITEM_ID.to_string(),
            owner: false,
            permission: 0,
            share_role_id: "1".to_string(),
            content: None,
            content_key_rotation: None,
            content_format_version: None,
            expiration_time: None,
            create_time: 8765432,
            group_id: None,
        };
        let share_response_2_clone = share_response_2.clone();
        let handled = server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
            success(GetSharesResponse {
                shares: vec![
                    share_response_1_clone.clone(),
                    share_response_2_clone.clone(),
                ],
            })
        });

        let recorder = server.new_recorder();

        let shares = client
            .list_shares()
            .await
            .expect("Should be able to list shares");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        assert_eq!(2, shares.len());
        assert_share_matches_response(&shares[0], &share_response_1);
        assert_share_matches_response(&shares[1], &share_response_2);
    }

    fn assert_share_matches_response(share: &Share, response: &ShareResponse) {
        assert_eq!(response.share_id, share.id.value());
        assert_eq!(response.address_id, share.address_id.value());
        assert_eq!(response.vault_id, share.vault_id.value());

        match response.group_id {
            Some(ref id) => match share.group_id {
                Some(ref gid) => assert_eq!(id, gid.value()),
                None => panic!("GroupID does not match"),
            },
            None => assert!(share.group_id.is_none()),
        }

        match &share.share_type {
            ShareType::Vault { vault_id } => {
                assert_eq!(vault_id.value(), response.vault_id);
                assert_eq!(vault_id.value(), response.target_id);
                assert_eq!(TargetType::Vault.value(), response.target_type);
            }
            ShareType::Item { item_id, vault_id } => {
                assert_eq!(vault_id.value(), response.vault_id);
                assert_eq!(item_id.value(), response.target_id);
                assert_eq!(TargetType::Item.value(), response.target_type);
            }
        }

        match response.content {
            Some(_) => assert!(share.content.is_some()),
            None => assert!(share.content.is_none()),
        }
    }
}
