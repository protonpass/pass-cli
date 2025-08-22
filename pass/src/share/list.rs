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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
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
