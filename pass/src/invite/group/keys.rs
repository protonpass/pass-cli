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
pub(crate) struct GroupResponse {
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
pub(crate) struct GetGroupsResponse {
    #[serde(rename = "Groups")]
    pub groups: Vec<GroupResponse>,
}

impl PassClient {
    pub async fn get_group_addresses(&self) -> Result<Vec<GroupAddress>> {
        let res = self
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_tools::*;
    use std::sync::Arc;

    use muon::rest::core::v4::addresses;
    use muon::test::server::{HTTP, Server};

    #[muon::test(scheme(HTTP))]
    async fn test_get_group_addresses_empty(server: Arc<Server>) {
        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/core/v4/groups", move |_| {
            success(GetGroupsResponse { groups: vec![] })
        });

        let recorder = server.new_recorder();
        let group_addresses = client
            .get_group_addresses()
            .await
            .expect("Should be able to get group addresses");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        assert!(group_addresses.is_empty());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_get_group_addresses_with_content(server: Arc<Server>) {
        const GROUP_1_ID: &str = "Group1ID";
        const GROUP_1_NAME: &str = "Test Group 1";
        const GROUP_1_DESCRIPTION: &str = "First test group";
        const GROUP_1_ADDRESS_ID: &str = "Address1ID";
        const GROUP_1_ADDRESS_EMAIL: &str = "group1@test.com";
        const GROUP_1_PERMISSIONS: i64 = 1;
        const GROUP_1_CREATE_TIME: i64 = 1234567890;
        const GROUP_1_FLAGS: i64 = 0;
        const GROUP_1_GROUP_VISIBILITY: i64 = 1;
        const GROUP_1_MEMBER_VISIBILITY: i64 = 1;

        const GROUP_2_ID: &str = "Group2ID";
        const GROUP_2_NAME: &str = "Test Group 2";
        const GROUP_2_DESCRIPTION: &str = "Second test group";
        const GROUP_2_ADDRESS_ID: &str = "Address2ID";
        const GROUP_2_ADDRESS_EMAIL: &str = "group2@test.com";
        const GROUP_2_PERMISSIONS: i64 = 2;
        const GROUP_2_CREATE_TIME: i64 = 1234567891;
        const GROUP_2_FLAGS: i64 = 1;
        const GROUP_2_GROUP_VISIBILITY: i64 = 2;
        const GROUP_2_MEMBER_VISIBILITY: i64 = 2;

        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/core/v4/groups", move |_| {
            success(GetGroupsResponse {
                groups: vec![
                    GroupResponse {
                        id: GROUP_1_ID.to_string(),
                        name: GROUP_1_NAME.to_string(),
                        address: Some(addresses::Address {
                            id: GROUP_1_ADDRESS_ID.to_string(),
                            email: GROUP_1_ADDRESS_EMAIL.to_string(),
                            keys: vec![],
                        }),
                        permissions: GROUP_1_PERMISSIONS,
                        create_time: GROUP_1_CREATE_TIME,
                        flags: GROUP_1_FLAGS,
                        group_visibility: GROUP_1_GROUP_VISIBILITY,
                        member_visibility: GROUP_1_MEMBER_VISIBILITY,
                        description: GROUP_1_DESCRIPTION.to_string(),
                    },
                    GroupResponse {
                        id: GROUP_2_ID.to_string(),
                        name: GROUP_2_NAME.to_string(),
                        address: Some(addresses::Address {
                            id: GROUP_2_ADDRESS_ID.to_string(),
                            email: GROUP_2_ADDRESS_EMAIL.to_string(),
                            keys: vec![],
                        }),
                        permissions: GROUP_2_PERMISSIONS,
                        create_time: GROUP_2_CREATE_TIME,
                        flags: GROUP_2_FLAGS,
                        group_visibility: GROUP_2_GROUP_VISIBILITY,
                        member_visibility: GROUP_2_MEMBER_VISIBILITY,
                        description: GROUP_2_DESCRIPTION.to_string(),
                    },
                ],
            })
        });

        let recorder = server.new_recorder();
        let group_addresses = client
            .get_group_addresses()
            .await
            .expect("Should be able to get group addresses");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        // Assert we got exactly 2 group addresses
        assert_eq!(2, group_addresses.len());

        // Assert first group address
        let group_1 = &group_addresses[0];
        assert_eq!(GROUP_1_ID, group_1.group_id.value());
        assert_eq!(GROUP_1_ADDRESS_ID, group_1.address.id.value());
        assert_eq!(GROUP_1_ADDRESS_EMAIL, group_1.address.email);
        assert!(group_1.address.keys.is_empty());

        // Assert second group address
        let group_2 = &group_addresses[1];
        assert_eq!(GROUP_2_ID, group_2.group_id.value());
        assert_eq!(GROUP_2_ADDRESS_ID, group_2.address.id.value());
        assert_eq!(GROUP_2_ADDRESS_EMAIL, group_2.address.email);
        assert!(group_2.address.keys.is_empty());
    }

    #[muon::test(scheme(HTTP))]
    async fn test_get_group_addresses_filters_groups_without_addresses(server: Arc<Server>) {
        const GROUP_WITH_ADDRESS_ID: &str = "GroupWithAddressID";
        const GROUP_WITH_ADDRESS_NAME: &str = "Group With Address";
        const GROUP_WITH_ADDRESS_DESCRIPTION: &str = "This group has an address";
        const GROUP_ADDRESS_ID: &str = "AddressID";
        const GROUP_ADDRESS_EMAIL: &str = "group@test.com";

        const GROUP_WITHOUT_ADDRESS_ID: &str = "GroupWithoutAddressID";
        const GROUP_WITHOUT_ADDRESS_NAME: &str = "Group Without Address";
        const GROUP_WITHOUT_ADDRESS_DESCRIPTION: &str = "This group has no address";

        let client = server.pass_client().await;

        let handled = server.handler_with_method(Method::GET, "/core/v4/groups", move |_| {
            success(GetGroupsResponse {
                groups: vec![
                    GroupResponse {
                        id: GROUP_WITH_ADDRESS_ID.to_string(),
                        name: GROUP_WITH_ADDRESS_NAME.to_string(),
                        address: Some(addresses::Address {
                            id: GROUP_ADDRESS_ID.to_string(),
                            email: GROUP_ADDRESS_EMAIL.to_string(),
                            keys: vec![],
                        }),
                        permissions: 1,
                        create_time: 1234567890,
                        flags: 0,
                        group_visibility: 1,
                        member_visibility: 1,
                        description: GROUP_WITH_ADDRESS_DESCRIPTION.to_string(),
                    },
                    GroupResponse {
                        id: GROUP_WITHOUT_ADDRESS_ID.to_string(),
                        name: GROUP_WITHOUT_ADDRESS_NAME.to_string(),
                        address: None, // No address
                        permissions: 1,
                        create_time: 1234567891,
                        flags: 0,
                        group_visibility: 1,
                        member_visibility: 1,
                        description: GROUP_WITHOUT_ADDRESS_DESCRIPTION.to_string(),
                    },
                ],
            })
        });

        let recorder = server.new_recorder();
        let group_addresses = client
            .get_group_addresses()
            .await
            .expect("Should be able to get group addresses");

        assert_hit!(handled);
        let requests = recorder.read();
        assert_eq!(1, requests.len());

        // Only the group with an address should be returned
        assert_eq!(1, group_addresses.len());

        let group = &group_addresses[0];
        assert_eq!(GROUP_WITH_ADDRESS_ID, group.group_id.value());
        assert_eq!(GROUP_ADDRESS_ID, group.address.id.value());
        assert_eq!(GROUP_ADDRESS_EMAIL, group.address.email);
        assert!(group.address.keys.is_empty());
    }
}
