use crate::item::item_keys::DecryptedItemKey;
use crate::item::list::ItemRevision;
use crate::share::keys::{ShareKeyList, ShareKeyResponse};
use crate::test_tools::{MuonServerExt, TEST_ADDRESS_ID, success};
use muon::Method;
use muon::test::server::Server;
pub use pass_domain::utils::random_string;
use pass_domain::{
    CustomItem, CustomSection, ItemContent, ItemData, ItemExtraField, ItemExtraFieldContent,
    TargetType, crypto,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[macro_export]
macro_rules! share_id {
    ($id:expr) => {
        pass_domain::ShareId::new($id.to_string())
    };
}

#[macro_export]
macro_rules! vault_id {
    ($id:expr) => {
        pass_domain::VaultId::new($id.to_string())
    };
}

#[macro_export]
macro_rules! item_id {
    ($id:expr) => {
        pass_domain::ItemId::new($id.to_string())
    };
}

#[macro_export]
macro_rules! address_id {
    ($id:expr) => {
        pass_domain::AddressId::new($id.to_string())
    };
}

#[macro_export]
macro_rules! group_id {
    ($id:expr) => {
        pass_domain::GroupId::new($id.to_string())
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

pub fn setup_item_revision(
    server: &Arc<Server>,
    share_id: &str,
    item_id: &str,
    item_revision: ItemRevision,
) -> Arc<AtomicBool> {
    server.handler_with_method(
        Method::GET,
        format!("/pass/v1/share/{share_id}/item/{item_id}"),
        move |_| {
            success(crate::item::get_one::GetItemResponse {
                item: item_revision.clone(),
            })
        },
    )
}

pub struct ItemRevisionBuilder {
    item_id: String,
    revision: Option<u64>,
    content_format_version: Option<i32>,
    content: Option<String>,
    item_key: Option<Option<String>>,
    state: Option<u8>,
    flags: Option<u64>,
    alias_email: Option<Option<String>>,
}

#[allow(dead_code)]
impl ItemRevisionBuilder {
    pub fn new(item_id: String) -> Self {
        Self {
            item_id,
            revision: None,
            content_format_version: None,
            content: None,
            item_key: None,
            state: None,
            flags: None,
            alias_email: None,
        }
    }

    pub fn with_revision(mut self, value: u64) -> Self {
        self.revision = Some(value);
        self
    }
    pub fn with_content_format_version(mut self, value: i32) -> Self {
        self.content_format_version = Some(value);
        self
    }
    pub fn with_content(mut self, value: String) -> Self {
        self.content = Some(value);
        self
    }
    pub fn with_item_key(mut self, value: Option<String>) -> Self {
        self.item_key = Some(value);
        self
    }
    pub fn with_state(mut self, value: u8) -> Self {
        self.state = Some(value);
        self
    }
    pub fn with_flags(mut self, value: u64) -> Self {
        self.flags = Some(value);
        self
    }
    pub fn with_alias_email(mut self, value: Option<String>) -> Self {
        self.alias_email = Some(value);
        self
    }

    pub fn build(self) -> ItemRevision {
        ItemRevision {
            item_id: self.item_id,
            revision: self.revision.unwrap_or(1),
            content_format_version: self.content_format_version.unwrap_or(1),
            key_rotation: 1,
            content: self.content.unwrap_or_default(),
            item_key: self.item_key.unwrap_or(None),
            state: self.state.unwrap_or(1),
            flags: self.flags.unwrap_or(0),
            alias_email: self.alias_email.unwrap_or(None),
        }
    }
}

pub fn encrypt_for_vault_key(data: &[u8], tag: crypto::EncryptionTag) -> Vec<u8> {
    crypto::encrypt(data, &TEST_SHARE_KEY, tag).expect("encrypt data failed")
}

#[allow(dead_code)]
pub fn decrypt_for_vault_key(ciphertext: &[u8], tag: crypto::EncryptionTag) -> Vec<u8> {
    crypto::decrypt(ciphertext, &TEST_SHARE_KEY, tag).expect("decrypt data failed")
}

#[allow(dead_code)]
pub struct EncryptItemContentsResult {
    pub item_key: DecryptedItemKey,
    pub encrypted_item_key: Vec<u8>,
    pub encrypted_contents: Vec<u8>,
}

pub fn encrypt_item_contents(data: ItemData) -> EncryptItemContentsResult {
    let serialized = data.serialize().expect("serialize data failed");
    let item_key = crypto::generate_encryption_key();
    let encrypted_data =
        crypto::encrypt(&serialized, &item_key, crypto::EncryptionTag::ItemContent)
            .expect("Error encrypting item content");

    let encrypted_item_key = encrypt_for_vault_key(&item_key, crypto::EncryptionTag::ItemKey);

    EncryptItemContentsResult {
        item_key: DecryptedItemKey(item_key),
        encrypted_item_key,
        encrypted_contents: encrypted_data,
    }
}

pub fn create_random_item() -> ItemData {
    ItemData::new(
        random_string(10),
        random_string(10),
        random_string(10),
        ItemContent::Custom(CustomItem {
            sections: vec![CustomSection {
                section_name: random_string(10),
                section_fields: vec![ItemExtraField {
                    name: random_string(10),
                    content: ItemExtraFieldContent::Text(random_string(10)),
                }],
            }],
        }),
        vec![],
    )
    .expect("Error creating item data")
}
