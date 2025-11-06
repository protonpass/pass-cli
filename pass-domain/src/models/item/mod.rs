mod attachment;
mod field;
mod flags;

use crate::protos::item::item_v1;
use crate::{ShareId, VaultId};
use anyhow::{Context, Result, anyhow};
pub use attachment::*;
pub use flags::*;
use protobuf::Message;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ItemState {
    Active = 1,
    Trashed = 2,
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Item {
    pub id: ItemId,
    pub share_id: ShareId,
    pub vault_id: VaultId,
    pub content: ItemData,
    pub state: ItemState,
    pub flags: Vec<ItemFlag>,
    pub create_time: chrono::NaiveDateTime,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

    pub fn pretty_print(&self) -> String {
        let mut out = String::new();

        let item_content = self.content.pretty_print();
        if !item_content.is_empty() {
            out.push_str(&item_content);
        }

        if !self.extra_fields.is_empty() {
            out.push_str("\n\nExtra fields:\n");
            for field in &self.extra_fields {
                out.push_str(format!("  - {}: {:?}", field.name, field.content).as_str());
            }
        }

        out
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let as_proto =
            item_v1::Item::parse_from_bytes(data).context("Error decoding Item from proto")?;

        Ok(Self::from(as_proto))
    }

    pub fn generate_uuid() -> String {
        uuid::Uuid::new_v4().to_string()
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

impl ItemContent {
    pub fn pretty_print(&self) -> String {
        match self {
            ItemContent::Note(v) => v.pretty_print(),
            ItemContent::Login(v) => v.pretty_print(),
            ItemContent::Alias(v) => v.pretty_print(),
            ItemContent::CreditCard(v) => v.pretty_print(),
            ItemContent::Identity(v) => v.pretty_print(),
            ItemContent::SshKey(v) => v.pretty_print(),
            ItemContent::Wifi(v) => v.pretty_print(),
            ItemContent::Custom(v) => v.pretty_print(),
        }
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct NoteItem;

impl NoteItem {
    pub fn pretty_print(&self) -> String {
        String::new()
    }
}

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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct LoginItem {
    pub email: String,
    pub username: String,
    pub password: String,
    pub urls: Vec<String>,
    pub totp_uri: String,
}

impl LoginItem {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();

        if !self.email.is_empty() {
            out.push_str(format!("\n - Email: {}", self.email).as_str());
        }
        if !self.username.is_empty() {
            out.push_str(format!("\n - Username: {}", self.username).as_str());
        }
        if !self.password.is_empty() {
            out.push_str(format!("\n - Password: {}", self.password).as_str());
        }
        if !self.totp_uri.is_empty() {
            out.push_str(format!("\n - TOTP URI: {}", self.totp_uri).as_str());
        }
        if !self.urls.is_empty() {
            out.push_str("\n - URLS");
            for url in &self.urls {
                out.push_str(format!("\n   - {}", url).as_str());
            }
        }

        out
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct AliasItem;

impl AliasItem {
    pub fn pretty_print(&self) -> String {
        String::new()
    }
}

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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CreditCardItem {
    pub cardholder_name: String,
    pub card_type: CardType,
    pub number: String,
    pub verification_number: String,
    pub expiration_date: String,
    pub pin: String,
}

impl CreditCardItem {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        if !self.cardholder_name.is_empty() {
            out.push_str(format!("\n- Cardholder name: {}", self.cardholder_name).as_str());
        }
        if !self.number.is_empty() {
            out.push_str(format!("\n- Number: {}", self.number).as_str());
        }
        if !self.verification_number.is_empty() {
            out.push_str(format!("\n- Verification number: {}", self.verification_number).as_str());
        }
        if !self.expiration_date.is_empty() {
            out.push_str(format!("\n- Expiration date: {}", self.expiration_date).as_str());
        }
        if !self.pin.is_empty() {
            out.push_str(format!("\n- PIN: {}", self.pin).as_str());
        }
        out
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

impl IdentityItem {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        if !self.full_name.is_empty() {
            out.push_str(format!("\n- Full name: {}", self.full_name).as_str());
        }
        if !self.email.is_empty() {
            out.push_str(format!("\n- Email: {}", self.email).as_str());
        }
        if !self.phone_number.is_empty() {
            out.push_str(format!("\n- Phone number: {}", self.phone_number).as_str());
        }
        if !self.first_name.is_empty() {
            out.push_str(format!("\n- First name: {}", self.first_name).as_str());
        }
        if !self.middle_name.is_empty() {
            out.push_str(format!("\n- Middle name: {}", self.middle_name).as_str());
        }
        if !self.last_name.is_empty() {
            out.push_str(format!("\n- Last name: {}", self.last_name).as_str());
        }
        if !self.birthdate.is_empty() {
            out.push_str(format!("\n- Birthdate: {}", self.birthdate).as_str());
        }
        if !self.gender.is_empty() {
            out.push_str(format!("\n- Gender: {}", self.gender).as_str());
        }
        if !self.organization.is_empty() {
            out.push_str(format!("\n- Organization: {}", self.organization).as_str());
        }
        if !self.street_address.is_empty() {
            out.push_str(format!("\n- Street address: {}", self.street_address).as_str());
        }
        if !self.zip_or_postal_code.is_empty() {
            out.push_str(format!("\n- Zip or postal code: {}", self.zip_or_postal_code).as_str());
        }
        if !self.city.is_empty() {
            out.push_str(format!("\n- City: {}", self.city).as_str());
        }
        if !self.state_or_province.is_empty() {
            out.push_str(format!("\n- State or province: {}", self.state_or_province).as_str());
        }
        if !self.country_or_region.is_empty() {
            out.push_str(format!("\n- Country or region: {}", self.country_or_region).as_str());
        }
        if !self.social_security_number.is_empty() {
            out.push_str(
                format!(
                    "\n- Social security number: {}",
                    self.social_security_number
                )
                .as_str(),
            );
        }
        if !self.passport_number.is_empty() {
            out.push_str(format!("\n- Passport number: {}", self.passport_number).as_str());
        }
        if !self.license_number.is_empty() {
            out.push_str(format!("\n- License number: {}", self.license_number).as_str());
        }
        if !self.website.is_empty() {
            out.push_str(format!("\n- Website: {}", self.website).as_str());
        }
        if !self.company.is_empty() {
            out.push_str(format!("\n- Company: {}", self.company).as_str());
        }
        if !self.job_title.is_empty() {
            out.push_str(format!("\n- Job title: {}", self.job_title).as_str());
        }
        out
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct SshKeyItem {
    pub private_key: String,
    pub public_key: String,
}

impl SshKeyItem {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        if !self.public_key.is_empty() {
            out.push_str(format!("\n- Public key: {}", self.public_key).as_str());
        }
        if !self.private_key.is_empty() {
            out.push_str(format!("\n- Private key:\n{}", self.private_key).as_str());
        }
        out
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct WifiItem {
    pub ssid: String,
    pub password: String,
    pub security: WifiSecurity,
}

impl WifiItem {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        if !self.ssid.is_empty() {
            out.push_str(format!("\n- SSID: {}", self.ssid).as_str());
        }
        if !self.password.is_empty() {
            out.push_str(format!("\n- Password: {}", self.password).as_str());
        }
        if self.security != WifiSecurity::UnspecifiedWifiSecurity {
            out.push_str(format!("\n- Wifi Security: {:?}", self.security).as_str());
        }
        out
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct CustomItem {
    pub sections: Vec<CustomSection>,
}

impl CustomItem {
    pub fn pretty_print(&self) -> String {
        let mut out = String::new();
        if !self.sections.is_empty() {
            for section in &self.sections {
                out.push_str(format!("\n- Section: {}", section.section_name).as_str());
                if !section.section_fields.is_empty() {
                    for field in &section.section_fields {
                        out.push_str(format!("\n  - {}: {:?}", field.name, field.content).as_str());
                    }
                }
            }
        }
        out
    }
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

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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
