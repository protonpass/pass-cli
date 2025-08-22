use crate::PassClient;
use anyhow::{Context, Result};
use muon::GET;
use muon::rest::core::v4::addresses;
use pass_domain::{Address, GroupId};
use serde::{Deserialize, Serialize};

pub struct GroupAddress {
    pub group_id: GroupId,
    pub address: Address,
}

#[derive(Serialize, Deserialize)]
struct GroupResponse {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Address")]
    pub address: Option<addresses::Address>,
    #[serde(rename = "Permissions")]
    pub permissions: i64,
    #[serde(rename = "CreateTime")]
    pub create_time: i64,
    #[serde(rename = "Flags")]
    pub flags: i64,
    #[serde(rename = "GroupVisibility")]
    pub group_visibility: i64,
    #[serde(rename = "MemberVisibility")]
    pub member_visibility: i64,
    #[serde(rename = "Description")]
    pub description: String,
}

#[derive(Serialize, Deserialize)]
struct GetGroupsResponse {
    #[serde(rename = "Groups")]
    pub groups: Vec<GroupResponse>,
}

impl PassClient {
    pub async fn get_group_addresses(&self) -> Result<Vec<GroupAddress>> {
        let res = self
            .client
            .send(GET!("/core/v4/groups"))
            .await
            .context("Error fetching groups")?;
        let response: GetGroupsResponse = assert_response!(res);

        let mut result = vec![];

        for group in response.groups {
            if let Some(address) = group.address {
                result.push(GroupAddress {
                    group_id: GroupId::new(group.id),
                    address: crate::account::address::api_address_to_domain_address(address),
                });
            }
        }

        Ok(result)
    }
}
