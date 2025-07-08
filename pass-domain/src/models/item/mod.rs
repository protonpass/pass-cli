mod attachment;
mod field;
mod flags;

use crate::protos::item::item_v1;
use crate::{ShareId, VaultId};
use anyhow::{Context, Result, anyhow};
pub use attachment::*;
pub use flags::*;
use protobuf::Message;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ItemId(pub(crate) String);
display_for_basic!(ItemId);

impl ItemId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ItemState {
    Active,
    Trashed,
}

impl TryFrom<u8> for ItemState {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(ItemState::Active),
            2 => Ok(ItemState::Trashed),
            _ => Err(anyhow!("Invalid item state value: {}", value)),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Item {
    pub id: ItemId,
    pub share_id: ShareId,
    pub vault_id: VaultId,
    pub content: ItemData,
    pub state: ItemState,
    pub flags: Vec<ItemFlag>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ItemData {
    pub title: String,
    pub note: String,
    pub item_uuid: String,
    pub content: ItemContent,
    pub extra_fields: Vec<ItemExtraField>,
}

impl ItemData {
    pub fn new(
        title: String,
        note: String,
        item_uuid: String,
        content: ItemContent,
        extra_fields: Vec<ItemExtraField>,
    ) -> Result<Self> {
        if title.is_empty() {
            return Err(anyhow!("The item title cannot be empty."));
        }

        Ok(Self {
            title,
            note,
            item_uuid,
            content,
            extra_fields,
        })
    }

    pub fn serialize(self) -> Result<Vec<u8>> {
        let as_proto = item_v1::Item::from(self);
        as_proto
            .write_to_bytes()
            .map_err(|e| anyhow!("Error serializing item to proto: {}", e))
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let as_proto =
            item_v1::Item::parse_from_bytes(data).context("Error decoding Item from proto")?;

        Ok(Self::from(as_proto))
    }
}

impl From<ItemData> for item_v1::Item {
    fn from(value: ItemData) -> Self {
        let metadata = item_v1::Metadata {
            name: value.title,
            note: value.note,
            item_uuid: value.item_uuid,
            ..Default::default()
        };

        let content = item_v1::Content {
            content: Some(item_v1::content::Content::from(value.content)),
            ..Default::default()
        };

        item_v1::Item {
            metadata: protobuf::MessageField::some(metadata),
            content: protobuf::MessageField::some(content),
            extra_fields: value
                .extra_fields
                .into_iter()
                .map(item_v1::ExtraField::from)
                .collect(),
            ..Default::default()
        }
    }
}

impl From<item_v1::Item> for ItemData {
    fn from(value: item_v1::Item) -> Self {
        let metadata = value.metadata.unwrap_or_default();
        let content = value.content.unwrap_or_default();

        Self {
            title: metadata.name,
            note: metadata.note,
            item_uuid: metadata.item_uuid,
            content: ItemContent::from(content),
            extra_fields: value
                .extra_fields
                .into_iter()
                .map(ItemExtraField::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ItemExtraField {
    pub name: String,
    pub content: ItemExtraFieldContent,
}

impl From<ItemExtraField> for item_v1::ExtraField {
    fn from(value: ItemExtraField) -> Self {
        item_v1::ExtraField {
            field_name: value.name,
            content: Some(item_v1::extra_field::Content::from(value.content)),
            ..Default::default()
        }
    }
}

impl From<item_v1::ExtraField> for ItemExtraField {
    fn from(value: item_v1::ExtraField) -> Self {
        Self {
            name: value.field_name,
            content: value
                .content
                .map(ItemExtraFieldContent::from)
                .unwrap_or(ItemExtraFieldContent::Text(String::new())),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum ItemExtraFieldContent {
    Text(String),
    Totp(String),
    Hidden(String),
    Timestamp(i64), // Unix timestamp in seconds
}

impl From<ItemExtraFieldContent> for item_v1::extra_field::Content {
    fn from(value: ItemExtraFieldContent) -> Self {
        match value {
            ItemExtraFieldContent::Text(content) => {
                item_v1::extra_field::Content::Text(item_v1::ExtraTextField {
                    content,
                    ..Default::default()
                })
            }
            ItemExtraFieldContent::Totp(totp_uri) => {
                item_v1::extra_field::Content::Totp(item_v1::ExtraTotp {
                    totp_uri,
                    ..Default::default()
                })
            }
            ItemExtraFieldContent::Hidden(content) => {
                item_v1::extra_field::Content::Hidden(item_v1::ExtraHiddenField {
                    content,
                    ..Default::default()
                })
            }
            ItemExtraFieldContent::Timestamp(timestamp) => {
                let proto_timestamp = protobuf::well_known_types::timestamp::Timestamp {
                    seconds: timestamp,
                    nanos: 0,
                    ..Default::default()
                };
                item_v1::extra_field::Content::Timestamp(item_v1::ExtraTimestampField {
                    timestamp: protobuf::MessageField::some(proto_timestamp),
                    ..Default::default()
                })
            }
        }
    }
}

impl From<item_v1::extra_field::Content> for ItemExtraFieldContent {
    fn from(value: item_v1::extra_field::Content) -> Self {
        match value {
            item_v1::extra_field::Content::Text(text_field) => {
                ItemExtraFieldContent::Text(text_field.content)
            }
            item_v1::extra_field::Content::Totp(totp_field) => {
                ItemExtraFieldContent::Totp(totp_field.totp_uri)
            }
            item_v1::extra_field::Content::Hidden(hidden_field) => {
                ItemExtraFieldContent::Hidden(hidden_field.content)
            }
            item_v1::extra_field::Content::Timestamp(timestamp_field) => {
                let timestamp = timestamp_field.timestamp.unwrap_or_default();
                ItemExtraFieldContent::Timestamp(timestamp.seconds)
            }
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub enum ItemContent {
    Note(NoteItem),
    Login(LoginItem),
    Alias(AliasItem),
    CreditCard(CreditCardItem),
    Identity(Box<IdentityItem>),
    SshKey(SshKeyItem),
    Wifi(WifiItem),
    Custom(CustomItem),
}

impl From<ItemContent> for item_v1::content::Content {
    fn from(value: ItemContent) -> Self {
        match value {
            ItemContent::Note(note) => {
                item_v1::content::Content::Note(item_v1::ItemNote::from(note))
            }
            ItemContent::Login(login) => {
                item_v1::content::Content::Login(item_v1::ItemLogin::from(login))
            }
            ItemContent::Alias(alias) => {
                item_v1::content::Content::Alias(item_v1::ItemAlias::from(alias))
            }
            ItemContent::CreditCard(credit_card) => {
                item_v1::content::Content::CreditCard(item_v1::ItemCreditCard::from(credit_card))
            }
            ItemContent::Identity(identity) => {
                item_v1::content::Content::Identity(item_v1::ItemIdentity::from(*identity))
            }
            ItemContent::SshKey(ssh_key) => {
                item_v1::content::Content::SshKey(item_v1::ItemSSHKey::from(ssh_key))
            }
            ItemContent::Wifi(wifi) => {
                item_v1::content::Content::Wifi(item_v1::ItemWifi::from(wifi))
            }
            ItemContent::Custom(custom) => {
                item_v1::content::Content::Custom(item_v1::ItemCustom::from(custom))
            }
        }
    }
}

impl From<item_v1::Content> for ItemContent {
    fn from(value: item_v1::Content) -> Self {
        match value.content {
            Some(item_v1::content::Content::Note(note)) => ItemContent::Note(NoteItem::from(note)),
            Some(item_v1::content::Content::Login(login)) => {
                ItemContent::Login(LoginItem::from(login))
            }
            Some(item_v1::content::Content::Alias(alias)) => {
                ItemContent::Alias(AliasItem::from(alias))
            }
            Some(item_v1::content::Content::CreditCard(credit_card)) => {
                ItemContent::CreditCard(CreditCardItem::from(credit_card))
            }
            Some(item_v1::content::Content::Identity(identity)) => {
                ItemContent::Identity(Box::new(IdentityItem::from(identity)))
            }
            Some(item_v1::content::Content::SshKey(ssh_key)) => {
                ItemContent::SshKey(SshKeyItem::from(ssh_key))
            }
            Some(item_v1::content::Content::Wifi(wifi)) => ItemContent::Wifi(WifiItem::from(wifi)),
            Some(item_v1::content::Content::Custom(custom)) => {
                ItemContent::Custom(CustomItem::from(custom))
            }
            None => ItemContent::Note(NoteItem),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct NoteItem;

impl From<NoteItem> for item_v1::ItemNote {
    fn from(_value: NoteItem) -> Self {
        item_v1::ItemNote::default()
    }
}

impl From<item_v1::ItemNote> for NoteItem {
    fn from(_value: item_v1::ItemNote) -> Self {
        NoteItem
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LoginItem {
    pub email: String,
    pub username: String,
    pub password: String,
    pub urls: Vec<String>,
    pub totp_uri: String,
}

impl From<LoginItem> for item_v1::ItemLogin {
    fn from(value: LoginItem) -> Self {
        item_v1::ItemLogin {
            item_email: value.email,
            item_username: value.username,
            password: value.password,
            urls: value.urls,
            totp_uri: value.totp_uri,
            ..Default::default()
        }
    }
}

impl From<item_v1::ItemLogin> for LoginItem {
    fn from(value: item_v1::ItemLogin) -> Self {
        Self {
            email: value.item_email,
            username: value.item_username,
            password: value.password,
            urls: value.urls,
            totp_uri: value.totp_uri,
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct AliasItem;

impl From<AliasItem> for item_v1::ItemAlias {
    fn from(_value: AliasItem) -> Self {
        item_v1::ItemAlias::default()
    }
}

impl From<item_v1::ItemAlias> for AliasItem {
    fn from(_value: item_v1::ItemAlias) -> Self {
        AliasItem
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CreditCardItem {
    pub cardholder_name: String,
    pub card_type: CardType,
    pub number: String,
    pub verification_number: String,
    pub expiration_date: String,
    pub pin: String,
}

impl From<CreditCardItem> for item_v1::ItemCreditCard {
    fn from(value: CreditCardItem) -> Self {
        item_v1::ItemCreditCard {
            cardholder_name: value.cardholder_name,
            card_type: item_v1::CardType::from(value.card_type).into(),
            number: value.number,
            verification_number: value.verification_number,
            expiration_date: value.expiration_date,
            pin: value.pin,
            ..Default::default()
        }
    }
}

impl From<item_v1::ItemCreditCard> for CreditCardItem {
    fn from(value: item_v1::ItemCreditCard) -> Self {
        Self {
            cardholder_name: value.cardholder_name,
            card_type: CardType::from(value.card_type.enum_value_or_default()),
            number: value.number,
            verification_number: value.verification_number,
            expiration_date: value.expiration_date,
            pin: value.pin,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct IdentityItem {
    pub full_name: String,
    pub email: String,
    pub phone_number: String,
    pub first_name: String,
    pub middle_name: String,
    pub last_name: String,
    pub birthdate: String,
    pub gender: String,
    pub organization: String,
    pub street_address: String,
    pub zip_or_postal_code: String,
    pub city: String,
    pub state_or_province: String,
    pub country_or_region: String,
    pub social_security_number: String,
    pub passport_number: String,
    pub license_number: String,
    pub website: String,
    pub company: String,
    pub job_title: String,
}

impl From<IdentityItem> for item_v1::ItemIdentity {
    fn from(value: IdentityItem) -> Self {
        item_v1::ItemIdentity {
            full_name: value.full_name,
            email: value.email,
            phone_number: value.phone_number,
            first_name: value.first_name,
            middle_name: value.middle_name,
            last_name: value.last_name,
            birthdate: value.birthdate,
            gender: value.gender,
            organization: value.organization,
            street_address: value.street_address,
            zip_or_postal_code: value.zip_or_postal_code,
            city: value.city,
            state_or_province: value.state_or_province,
            country_or_region: value.country_or_region,
            social_security_number: value.social_security_number,
            passport_number: value.passport_number,
            license_number: value.license_number,
            website: value.website,
            company: value.company,
            job_title: value.job_title,
            ..Default::default()
        }
    }
}

impl From<item_v1::ItemIdentity> for IdentityItem {
    fn from(value: item_v1::ItemIdentity) -> Self {
        Self {
            full_name: value.full_name,
            email: value.email,
            phone_number: value.phone_number,
            first_name: value.first_name,
            middle_name: value.middle_name,
            last_name: value.last_name,
            birthdate: value.birthdate,
            gender: value.gender,
            organization: value.organization,
            street_address: value.street_address,
            zip_or_postal_code: value.zip_or_postal_code,
            city: value.city,
            state_or_province: value.state_or_province,
            country_or_region: value.country_or_region,
            social_security_number: value.social_security_number,
            passport_number: value.passport_number,
            license_number: value.license_number,
            website: value.website,
            company: value.company,
            job_title: value.job_title,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SshKeyItem {
    pub private_key: String,
    pub public_key: String,
}

impl From<SshKeyItem> for item_v1::ItemSSHKey {
    fn from(value: SshKeyItem) -> Self {
        item_v1::ItemSSHKey {
            private_key: value.private_key,
            public_key: value.public_key,
            ..Default::default()
        }
    }
}

impl From<item_v1::ItemSSHKey> for SshKeyItem {
    fn from(value: item_v1::ItemSSHKey) -> Self {
        Self {
            private_key: value.private_key,
            public_key: value.public_key,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct WifiItem {
    pub ssid: String,
    pub password: String,
    pub security: WifiSecurity,
}

impl From<WifiItem> for item_v1::ItemWifi {
    fn from(value: WifiItem) -> Self {
        item_v1::ItemWifi {
            ssid: value.ssid,
            password: value.password,
            security: item_v1::WifiSecurity::from(value.security).into(),
            ..Default::default()
        }
    }
}

impl From<item_v1::ItemWifi> for WifiItem {
    fn from(value: item_v1::ItemWifi) -> Self {
        Self {
            ssid: value.ssid,
            password: value.password,
            security: WifiSecurity::from(value.security.enum_value_or_default()),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct CustomItem {
    pub sections: Vec<CustomSection>,
}

impl From<CustomItem> for item_v1::ItemCustom {
    fn from(value: CustomItem) -> Self {
        item_v1::ItemCustom {
            sections: value
                .sections
                .into_iter()
                .map(item_v1::CustomSection::from)
                .collect(),
            ..Default::default()
        }
    }
}

impl From<item_v1::ItemCustom> for CustomItem {
    fn from(value: item_v1::ItemCustom) -> Self {
        Self {
            sections: value
                .sections
                .into_iter()
                .map(CustomSection::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CustomSection {
    pub section_name: String,
    pub section_fields: Vec<ItemExtraField>,
}

impl From<CustomSection> for item_v1::CustomSection {
    fn from(value: CustomSection) -> Self {
        item_v1::CustomSection {
            section_name: value.section_name,
            section_fields: value
                .section_fields
                .into_iter()
                .map(item_v1::ExtraField::from)
                .collect(),
            ..Default::default()
        }
    }
}

impl From<item_v1::CustomSection> for CustomSection {
    fn from(value: item_v1::CustomSection) -> Self {
        Self {
            section_name: value.section_name,
            section_fields: value
                .section_fields
                .into_iter()
                .map(ItemExtraField::from)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub enum CardType {
    #[default]
    Unspecified,
    Other,
    Visa,
    Mastercard,
    AmericanExpress,
}

impl From<CardType> for item_v1::CardType {
    fn from(value: CardType) -> Self {
        match value {
            CardType::Unspecified => item_v1::CardType::Unspecified,
            CardType::Other => item_v1::CardType::Other,
            CardType::Visa => item_v1::CardType::Visa,
            CardType::Mastercard => item_v1::CardType::Mastercard,
            CardType::AmericanExpress => item_v1::CardType::AmericanExpress,
        }
    }
}

impl From<item_v1::CardType> for CardType {
    fn from(value: item_v1::CardType) -> Self {
        match value {
            item_v1::CardType::Unspecified => CardType::Unspecified,
            item_v1::CardType::Other => CardType::Other,
            item_v1::CardType::Visa => CardType::Visa,
            item_v1::CardType::Mastercard => CardType::Mastercard,
            item_v1::CardType::AmericanExpress => CardType::AmericanExpress,
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub enum WifiSecurity {
    #[default]
    UnspecifiedWifiSecurity,
    WPA,
    WPA2,
    WPA3,
    WEP,
}

impl From<WifiSecurity> for item_v1::WifiSecurity {
    fn from(value: WifiSecurity) -> Self {
        match value {
            WifiSecurity::UnspecifiedWifiSecurity => item_v1::WifiSecurity::UnspecifiedWifiSecurity,
            WifiSecurity::WPA => item_v1::WifiSecurity::WPA,
            WifiSecurity::WPA2 => item_v1::WifiSecurity::WPA2,
            WifiSecurity::WPA3 => item_v1::WifiSecurity::WPA3,
            WifiSecurity::WEP => item_v1::WifiSecurity::WEP,
        }
    }
}

impl From<item_v1::WifiSecurity> for WifiSecurity {
    fn from(value: item_v1::WifiSecurity) -> Self {
        match value {
            item_v1::WifiSecurity::UnspecifiedWifiSecurity => WifiSecurity::UnspecifiedWifiSecurity,
            item_v1::WifiSecurity::WPA => WifiSecurity::WPA,
            item_v1::WifiSecurity::WPA2 => WifiSecurity::WPA2,
            item_v1::WifiSecurity::WPA3 => WifiSecurity::WPA3,
            item_v1::WifiSecurity::WEP => WifiSecurity::WEP,
        }
    }
}
