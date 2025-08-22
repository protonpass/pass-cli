use crate::share::keys::{ShareKeyList, ShareKeyResponse};
use crate::test_tools::{MuonServerExt, TEST_ADDRESS_ID, success};
use muon::Method;
use muon::test::server::Server;
use pass_domain::TargetType;
use std::sync::Arc;

#[macro_export]
macro_rules! share_id {
    ($id:expr) => {
        pass_domain::ShareId::new($id.to_string())
    };
}

#[macro_export]
macro_rules! item_id {
    ($id:expr) => {
        pass_domain::ItemId::new($id.to_string())
    };
}

pub const TEST_SHARE_KEY: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
];

// This is the base64(TEST_SHARE_KEY encrypted and signed with the primary user key)
pub const TEST_SHARE_KEY_ENCRYPTED: &str = "wV4D9Cy1x6t9568SAQdAQILzqjxAcyKkNsah9KqTmwFlcdBqB2zbsubdZ/vkLFYw+qty/+OlrqOyY3jQy5Z5KQZ2L1hqAu2ZpdIjP4TlhTNJR6sSimqk8XmHlvSRGrbj0sBdAR6CtbirCTAj6BJUQLkbiZKc9+T/oucnZi0RK1XpSzUE9jsWYaWhejE2Lzw33fVU3evpFEM+kfpAO5RBJMHFlCP51W/UKoCj9R0lOoCzhUMY1Bokyzfptykel/npbKuLH1lh/91oumJLXmGYUn43Qa0XjYHxV4V3OcX+xo1pOnQnroY7nkA6ZWwTbvIbNVriGDYOw7WF7+OfyQjMPxKXdGJu112P7HnSI6yfY5iQAj5BPcdnLg/Dr8LHo6l/OHBL3Pf09eVgYtytVreBOdtkXqzaPu+ppSsqaMU9KvjusEH6/HXZflDRrtRZdHD3HJ9FM3zZuyvzW07xuh5XSUVLxyhmYhNhE8lh6iWdEPbSJdSRSqmIe7iN2pRE0Aak";

pub const TEST_VAULT_ID: &str = "TEST_VAULT_ID";

pub fn setup_vault_share(server: &Arc<Server>, share_id: &str) {
    let share_id_string = share_id.to_string();
    let share_response = crate::share::list::ShareResponse {
        share_id: share_id_string.to_string(),
        address_id: TEST_ADDRESS_ID.to_string(),
        vault_id: TEST_VAULT_ID.to_string(),
        target_type: TargetType::Vault.value(),
        target_id: TEST_VAULT_ID.to_string(),
        owner: true,
        permission: 0,
        share_role_id: "1".to_string(),
        content: None,
        content_key_rotation: None,
        content_format_version: None,
        expiration_time: None,
        create_time: 0,
        group_id: None,
    };
    let share_response_clone = share_response.clone();
    server.handler_with_method(
        Method::GET,
        format!("/pass/v1/share/{}", share_id),
        move |_| success(share_response_clone.clone()),
    );
    server.handler_with_method(Method::GET, "/pass/v1/share", move |_| {
        success(crate::share::list::GetSharesResponse {
            shares: vec![share_response.clone()],
        })
    });
}

pub fn setup_share_keys(server: &Arc<Server>, share_id: &str) {
    server.handler_with_method(
        Method::GET,
        format!("/pass/v1/share/{}/key", share_id),
        move |_| {
            success(crate::share::keys::GetShareKeysResponse {
                keys: ShareKeyList {
                    keys: vec![ShareKeyResponse {
                        key_rotation: 1,
                        key: TEST_SHARE_KEY_ENCRYPTED.to_string(),
                        create_time: 123456789,
                    }],
                    total: 1,
                },
            })
        },
    );
}
