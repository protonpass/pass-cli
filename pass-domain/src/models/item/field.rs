use crate::{CreditCardItem, CustomItem, IdentityItem, LoginItem, SshKeyItem, WifiItem};
use crate::{Item, ItemContent, ItemExtraField, ItemExtraFieldContent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Text(String),
    Hidden(String),
    Totp(String),
}

impl Field {
    pub fn value(&self) -> String {
        match self {
            Field::Text(s) => s.clone(),
            Field::Hidden(s) => s.clone(),
            Field::Totp(s) => s.clone(),
        }
    }
}

impl ItemExtraField {
    pub fn value(&self) -> String {
        match &self.content {
            ItemExtraFieldContent::Text(text) => text.to_string(),
            ItemExtraFieldContent::Totp(totp) => totp.to_string(),
            ItemExtraFieldContent::Hidden(value) => value.to_string(),
            ItemExtraFieldContent::Timestamp(timestamp) => format!("{timestamp}"),
        }
    }

    pub fn as_field(&self) -> Field {
        match &self.content {
            ItemExtraFieldContent::Text(text) => Field::Text(text.to_string()),
            ItemExtraFieldContent::Totp(totp) => Field::Totp(totp.to_string()),
            ItemExtraFieldContent::Hidden(value) => Field::Hidden(value.to_string()),
            ItemExtraFieldContent::Timestamp(timestamp) => Field::Text(format!("{timestamp}")),
        }
    }
}

impl Item {
    pub fn get_field(&self, field: &str) -> Option<Field> {
        match field {
            "title" => Some(Field::Text(self.content.title.clone())),
            "note" => Some(Field::Text(self.content.note.clone())),
            _ => self.try_find_field(field),
        }
    }

    fn try_find_field(&self, field: &str) -> Option<Field> {
        for extra_field in self.content.extra_fields.iter() {
            if extra_field.name.to_lowercase() == field.to_lowercase() {
                return Some(extra_field.as_field());
            }
        }

        match &self.content.content {
            ItemContent::Note(_) => None,
            ItemContent::Alias(_) => None,

            ItemContent::Login(login) => self.find_login_field(login, field),
            ItemContent::CreditCard(cc) => self.find_cc_field(cc, field),
            ItemContent::Identity(identity) => self.find_identity_field(identity, field),
            ItemContent::SshKey(ssh) => self.find_ssh_field(ssh, field),
            ItemContent::Wifi(wifi) => self.find_wifi_field(wifi, field),
            ItemContent::Custom(custom) => self.find_custom_field(custom, field),
        }
    }

    fn find_login_field(&self, login: &LoginItem, field: &str) -> Option<Field> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "email" => Some(Field::Text(login.email.clone())),
            "username" => Some(Field::Text(login.username.clone())),
            "password" => Some(Field::Text(login.password.clone())),
            "totp_uri" | "totp" => Some(Field::Totp(login.totp_uri.clone())),
            "urls" | "url" => {
                if login.urls.is_empty() {
                    Some(Field::Text(String::new()))
                } else {
                    Some(Field::Text(login.urls.join(", ")))
                }
            }
            _ => None,
        }
    }

    fn find_cc_field(&self, cc: &CreditCardItem, field: &str) -> Option<Field> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "cardholder_name" | "cardholder" | "name" => {
                Some(Field::Text(cc.cardholder_name.clone()))
            }
            "card_type" | "type" => Some(Field::Text(format!("{:?}", cc.card_type))),
            "number" | "card_number" => Some(Field::Text(cc.number.clone())),
            "verification_number" | "cvv" | "cvc" => {
                Some(Field::Text(cc.verification_number.clone()))
            }
            "expiration_date" | "expiry" | "exp_date" => {
                Some(Field::Text(cc.expiration_date.clone()))
            }
            "pin" => Some(Field::Text(cc.pin.clone())),
            _ => None,
        }
    }

    fn find_identity_field(&self, identity: &IdentityItem, field: &str) -> Option<Field> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "full_name" | "fullname" => Some(Field::Text(identity.full_name.clone())),
            "email" => Some(Field::Text(identity.email.clone())),
            "phone_number" | "phone" => Some(Field::Text(identity.phone_number.clone())),
            "first_name" | "firstname" => Some(Field::Text(identity.first_name.clone())),
            "middle_name" | "middlename" => Some(Field::Text(identity.middle_name.clone())),
            "last_name" | "lastname" => Some(Field::Text(identity.last_name.clone())),
            "birthdate" | "birth_date" | "dob" => Some(Field::Text(identity.birthdate.clone())),
            "gender" => Some(Field::Text(identity.gender.clone())),
            "organization" | "org" => Some(Field::Text(identity.organization.clone())),
            "street_address" | "address" => Some(Field::Text(identity.street_address.clone())),
            "zip_or_postal_code" | "zip" | "postal_code" => {
                Some(Field::Text(identity.zip_or_postal_code.clone()))
            }
            "city" => Some(Field::Text(identity.city.clone())),
            "state_or_province" | "state" | "province" => {
                Some(Field::Text(identity.state_or_province.clone()))
            }
            "country_or_region" | "country" | "region" => {
                Some(Field::Text(identity.country_or_region.clone()))
            }
            "social_security_number" | "ssn" => {
                Some(Field::Text(identity.social_security_number.clone()))
            }
            "passport_number" | "passport" => Some(Field::Text(identity.passport_number.clone())),
            "license_number" | "license" => Some(Field::Text(identity.license_number.clone())),
            "website" | "url" => Some(Field::Text(identity.website.clone())),
            "company" => Some(Field::Text(identity.company.clone())),
            "job_title" | "title" => Some(Field::Text(identity.job_title.clone())),
            _ => None,
        }
    }

    fn find_ssh_field(&self, ssh: &SshKeyItem, field: &str) -> Option<Field> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "private_key" | "private" => Some(Field::Text(ssh.private_key.clone())),
            "public_key" | "public" => Some(Field::Text(ssh.public_key.clone())),
            _ => None,
        }
    }

    fn find_wifi_field(&self, wifi: &WifiItem, field: &str) -> Option<Field> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "ssid" => Some(Field::Text(wifi.ssid.clone())),
            "password" => Some(Field::Text(wifi.password.clone())),
            "security" => Some(Field::Text(format!("{:?}", wifi.security))),
            _ => None,
        }
    }

    fn find_custom_field(&self, custom: &CustomItem, field: &str) -> Option<Field> {
        for section in &custom.sections {
            for section_field in &section.section_fields {
                if section_field.name.to_lowercase() == field.to_lowercase() {
                    return Some(section_field.as_field());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AliasItem, CardType, CustomSection, ItemData, ItemId, ItemState, NoteItem, ShareId,
        VaultId, WifiSecurity,
    };

    // Helper functions to create default items for testing
    fn create_login_item() -> LoginItem {
        LoginItem {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password: "secretpass".to_string(),
            urls: vec![
                "https://example.com".to_string(),
                "https://test.com".to_string(),
            ],
            totp_uri: "otpauth://totp/test".to_string(),
        }
    }

    fn create_credit_card_item() -> CreditCardItem {
        CreditCardItem {
            cardholder_name: "John Doe".to_string(),
            card_type: CardType::Visa,
            number: "4111111111111111".to_string(),
            verification_number: "123".to_string(),
            expiration_date: "12/25".to_string(),
            pin: "1234".to_string(),
        }
    }

    fn create_identity_item() -> Box<IdentityItem> {
        Box::new(IdentityItem {
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            phone_number: "+1234567890".to_string(),
            first_name: "John".to_string(),
            middle_name: "Michael".to_string(),
            last_name: "Doe".to_string(),
            birthdate: "1990-01-01".to_string(),
            gender: "Male".to_string(),
            organization: "Test Corp".to_string(),
            street_address: "123 Main St".to_string(),
            zip_or_postal_code: "12345".to_string(),
            city: "Test City".to_string(),
            state_or_province: "Test State".to_string(),
            country_or_region: "Test Country".to_string(),
            social_security_number: "123-45-6789".to_string(),
            passport_number: "A1234567".to_string(),
            license_number: "D123456789".to_string(),
            website: "https://johndoe.com".to_string(),
            company: "Acme Inc".to_string(),
            job_title: "Software Engineer".to_string(),
        })
    }

    fn create_ssh_key_item() -> SshKeyItem {
        SshKeyItem {
            private_key: "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBg...".to_string(),
            public_key: "ssh-rsa AAAAB3NzaC1yc2E...".to_string(),
        }
    }

    fn create_wifi_item() -> WifiItem {
        WifiItem {
            ssid: "TestNetwork".to_string(),
            password: "wifipass123".to_string(),
            security: WifiSecurity::WPA2,
        }
    }

    fn create_custom_item() -> CustomItem {
        CustomItem {
            sections: vec![CustomSection {
                section_name: "Section1".to_string(),
                section_fields: vec![
                    ItemExtraField {
                        name: "custom_field1".to_string(),
                        content: ItemExtraFieldContent::Text("value1".to_string()),
                    },
                    ItemExtraField {
                        name: "custom_field2".to_string(),
                        content: ItemExtraFieldContent::Hidden("secret_value".to_string()),
                    },
                ],
            }],
        }
    }

    fn create_item_with_content(content: ItemContent) -> Item {
        Item {
            id: ItemId::new("test_id".to_string()),
            share_id: ShareId::new("share_id".to_string()),
            vault_id: VaultId::new("vault_id".to_string()),
            state: ItemState::Active,
            content: ItemData {
                title: "Test Item".to_string(),
                note: "Test note".to_string(),
                item_uuid: "uuid".to_string(),
                content,
                extra_fields: vec![ItemExtraField {
                    name: "extra_field".to_string(),
                    content: ItemExtraFieldContent::Text("extra_value".to_string()),
                }],
            },
            flags: vec![],
            create_time: chrono::DateTime::from_timestamp(1234567890, 0)
                .unwrap()
                .naive_utc(),
        }
    }

    #[test]
    fn test_get_field_basic_fields() {
        let item = create_item_with_content(ItemContent::Note(NoteItem));

        assert_eq!(
            item.get_field("title"),
            Some(Field::Text("Test Item".to_string()))
        );
        assert_eq!(
            item.get_field("note"),
            Some(Field::Text("Test note".to_string()))
        );
    }

    #[test]
    fn test_get_field_extra_fields() {
        let item = create_item_with_content(ItemContent::Note(NoteItem));

        assert_eq!(
            item.get_field("extra_field"),
            Some(Field::Text("extra_value".to_string()))
        );
        assert_eq!(
            item.get_field("EXTRA_FIELD"),
            Some(Field::Text("extra_value".to_string()))
        );
        assert_eq!(item.get_field("nonexistent"), None);
    }

    #[test]
    fn test_login_item_fields() {
        let item = create_item_with_content(ItemContent::Login(create_login_item()));

        assert_eq!(
            item.get_field("email"),
            Some(Field::Text("test@example.com".to_string()))
        );
        assert_eq!(
            item.get_field("EMAIL"),
            Some(Field::Text("test@example.com".to_string()))
        );
        assert_eq!(
            item.get_field("username"),
            Some(Field::Text("testuser".to_string()))
        );
        assert_eq!(
            item.get_field("password"),
            Some(Field::Text("secretpass".to_string()))
        );
        assert_eq!(
            item.get_field("totp_uri"),
            Some(Field::Totp("otpauth://totp/test".to_string()))
        );
        assert_eq!(
            item.get_field("totp"),
            Some(Field::Totp("otpauth://totp/test".to_string()))
        );
        assert_eq!(
            item.get_field("urls"),
            Some(Field::Text(
                "https://example.com, https://test.com".to_string()
            ))
        );
        assert_eq!(
            item.get_field("url"),
            Some(Field::Text(
                "https://example.com, https://test.com".to_string()
            ))
        );
    }

    #[test]
    fn test_credit_card_item_fields() {
        let item = create_item_with_content(ItemContent::CreditCard(create_credit_card_item()));

        assert_eq!(
            item.get_field("cardholder_name"),
            Some(Field::Text("John Doe".to_string()))
        );
        assert_eq!(
            item.get_field("cardholder"),
            Some(Field::Text("John Doe".to_string()))
        );
        assert_eq!(
            item.get_field("name"),
            Some(Field::Text("John Doe".to_string()))
        );
        assert_eq!(
            item.get_field("card_type"),
            Some(Field::Text("Visa".to_string()))
        );
        assert_eq!(
            item.get_field("type"),
            Some(Field::Text("Visa".to_string()))
        );
        assert_eq!(
            item.get_field("number"),
            Some(Field::Text("4111111111111111".to_string()))
        );
        assert_eq!(
            item.get_field("card_number"),
            Some(Field::Text("4111111111111111".to_string()))
        );
        assert_eq!(
            item.get_field("verification_number"),
            Some(Field::Text("123".to_string()))
        );
        assert_eq!(item.get_field("cvv"), Some(Field::Text("123".to_string())));
        assert_eq!(item.get_field("cvc"), Some(Field::Text("123".to_string())));
        assert_eq!(
            item.get_field("expiration_date"),
            Some(Field::Text("12/25".to_string()))
        );
        assert_eq!(
            item.get_field("expiry"),
            Some(Field::Text("12/25".to_string()))
        );
        assert_eq!(
            item.get_field("exp_date"),
            Some(Field::Text("12/25".to_string()))
        );
        assert_eq!(item.get_field("pin"), Some(Field::Text("1234".to_string())));
    }

    #[test]
    fn test_identity_item_fields() {
        let item = create_item_with_content(ItemContent::Identity(create_identity_item()));

        assert_eq!(
            item.get_field("full_name"),
            Some(Field::Text("John Doe".to_string()))
        );
        assert_eq!(
            item.get_field("fullname"),
            Some(Field::Text("John Doe".to_string()))
        );
        assert_eq!(
            item.get_field("email"),
            Some(Field::Text("john@example.com".to_string()))
        );
        assert_eq!(
            item.get_field("phone_number"),
            Some(Field::Text("+1234567890".to_string()))
        );
        assert_eq!(
            item.get_field("phone"),
            Some(Field::Text("+1234567890".to_string()))
        );
        assert_eq!(
            item.get_field("first_name"),
            Some(Field::Text("John".to_string()))
        );
        assert_eq!(
            item.get_field("firstname"),
            Some(Field::Text("John".to_string()))
        );
        assert_eq!(
            item.get_field("middle_name"),
            Some(Field::Text("Michael".to_string()))
        );
        assert_eq!(
            item.get_field("middlename"),
            Some(Field::Text("Michael".to_string()))
        );
        assert_eq!(
            item.get_field("last_name"),
            Some(Field::Text("Doe".to_string()))
        );
        assert_eq!(
            item.get_field("lastname"),
            Some(Field::Text("Doe".to_string()))
        );
        assert_eq!(
            item.get_field("birthdate"),
            Some(Field::Text("1990-01-01".to_string()))
        );
        assert_eq!(
            item.get_field("birth_date"),
            Some(Field::Text("1990-01-01".to_string()))
        );
        assert_eq!(
            item.get_field("dob"),
            Some(Field::Text("1990-01-01".to_string()))
        );
        assert_eq!(
            item.get_field("gender"),
            Some(Field::Text("Male".to_string()))
        );
        assert_eq!(
            item.get_field("organization"),
            Some(Field::Text("Test Corp".to_string()))
        );
        assert_eq!(
            item.get_field("org"),
            Some(Field::Text("Test Corp".to_string()))
        );
        assert_eq!(
            item.get_field("street_address"),
            Some(Field::Text("123 Main St".to_string()))
        );
        assert_eq!(
            item.get_field("address"),
            Some(Field::Text("123 Main St".to_string()))
        );
        assert_eq!(
            item.get_field("zip_or_postal_code"),
            Some(Field::Text("12345".to_string()))
        );
        assert_eq!(
            item.get_field("zip"),
            Some(Field::Text("12345".to_string()))
        );
        assert_eq!(
            item.get_field("postal_code"),
            Some(Field::Text("12345".to_string()))
        );
        assert_eq!(
            item.get_field("city"),
            Some(Field::Text("Test City".to_string()))
        );
        assert_eq!(
            item.get_field("state_or_province"),
            Some(Field::Text("Test State".to_string()))
        );
        assert_eq!(
            item.get_field("state"),
            Some(Field::Text("Test State".to_string()))
        );
        assert_eq!(
            item.get_field("province"),
            Some(Field::Text("Test State".to_string()))
        );
        assert_eq!(
            item.get_field("country_or_region"),
            Some(Field::Text("Test Country".to_string()))
        );
        assert_eq!(
            item.get_field("country"),
            Some(Field::Text("Test Country".to_string()))
        );
        assert_eq!(
            item.get_field("region"),
            Some(Field::Text("Test Country".to_string()))
        );
        assert_eq!(
            item.get_field("social_security_number"),
            Some(Field::Text("123-45-6789".to_string()))
        );
        assert_eq!(
            item.get_field("ssn"),
            Some(Field::Text("123-45-6789".to_string()))
        );
        assert_eq!(
            item.get_field("passport_number"),
            Some(Field::Text("A1234567".to_string()))
        );
        assert_eq!(
            item.get_field("passport"),
            Some(Field::Text("A1234567".to_string()))
        );
        assert_eq!(
            item.get_field("license_number"),
            Some(Field::Text("D123456789".to_string()))
        );
        assert_eq!(
            item.get_field("license"),
            Some(Field::Text("D123456789".to_string()))
        );
        assert_eq!(
            item.get_field("website"),
            Some(Field::Text("https://johndoe.com".to_string()))
        );
        assert_eq!(
            item.get_field("url"),
            Some(Field::Text("https://johndoe.com".to_string()))
        );
        assert_eq!(
            item.get_field("company"),
            Some(Field::Text("Acme Inc".to_string()))
        );
        assert_eq!(
            item.get_field("job_title"),
            Some(Field::Text("Software Engineer".to_string()))
        );
    }

    #[test]
    fn test_ssh_key_item_fields() {
        let item = create_item_with_content(ItemContent::SshKey(create_ssh_key_item()));

        assert_eq!(
            item.get_field("private_key"),
            Some(Field::Text(
                "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBg...".to_string()
            ))
        );
        assert_eq!(
            item.get_field("private"),
            Some(Field::Text(
                "-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBg...".to_string()
            ))
        );
        assert_eq!(
            item.get_field("public_key"),
            Some(Field::Text("ssh-rsa AAAAB3NzaC1yc2E...".to_string()))
        );
        assert_eq!(
            item.get_field("public"),
            Some(Field::Text("ssh-rsa AAAAB3NzaC1yc2E...".to_string()))
        );
    }

    #[test]
    fn test_wifi_item_fields() {
        let item = create_item_with_content(ItemContent::Wifi(create_wifi_item()));

        assert_eq!(
            item.get_field("ssid"),
            Some(Field::Text("TestNetwork".to_string()))
        );
        assert_eq!(
            item.get_field("password"),
            Some(Field::Text("wifipass123".to_string()))
        );
        assert_eq!(
            item.get_field("security"),
            Some(Field::Text("WPA2".to_string()))
        );
    }

    #[test]
    fn test_custom_item_fields() {
        let item = create_item_with_content(ItemContent::Custom(create_custom_item()));

        assert_eq!(
            item.get_field("custom_field1"),
            Some(Field::Text("value1".to_string()))
        );
        assert_eq!(
            item.get_field("CUSTOM_FIELD1"),
            Some(Field::Text("value1".to_string()))
        );
        assert_eq!(
            item.get_field("custom_field2"),
            Some(Field::Hidden("secret_value".to_string()))
        );
        assert_eq!(item.get_field("nonexistent_field"), None);
    }

    #[test]
    fn test_note_and_alias_items() {
        let note_item = create_item_with_content(ItemContent::Note(NoteItem));
        let alias_item = create_item_with_content(ItemContent::Alias(AliasItem));

        // Should only find basic fields and extra fields
        assert_eq!(
            note_item.get_field("title"),
            Some(Field::Text("Test Item".to_string()))
        );
        assert_eq!(
            note_item.get_field("note"),
            Some(Field::Text("Test note".to_string()))
        );
        assert_eq!(
            note_item.get_field("extra_field"),
            Some(Field::Text("extra_value".to_string()))
        );
        assert_eq!(note_item.get_field("nonexistent"), None);

        assert_eq!(
            alias_item.get_field("title"),
            Some(Field::Text("Test Item".to_string()))
        );
        assert_eq!(
            alias_item.get_field("note"),
            Some(Field::Text("Test note".to_string()))
        );
        assert_eq!(
            alias_item.get_field("extra_field"),
            Some(Field::Text("extra_value".to_string()))
        );
        assert_eq!(alias_item.get_field("nonexistent"), None);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let item = create_item_with_content(ItemContent::Login(create_login_item()));

        // Test various case combinations
        assert_eq!(
            item.get_field("EMAIL"),
            Some(Field::Text("test@example.com".to_string()))
        );
        assert_eq!(
            item.get_field("Email"),
            Some(Field::Text("test@example.com".to_string()))
        );
        assert_eq!(
            item.get_field("eMaIl"),
            Some(Field::Text("test@example.com".to_string()))
        );
        assert_eq!(
            item.get_field("USERNAME"),
            Some(Field::Text("testuser".to_string()))
        );
        assert_eq!(
            item.get_field("Password"),
            Some(Field::Text("secretpass".to_string()))
        );
        assert_eq!(
            item.get_field("TOTP_URI"),
            Some(Field::Totp("otpauth://totp/test".to_string()))
        );
    }

    #[test]
    fn test_empty_urls_in_login() {
        let mut login = create_login_item();
        login.urls = vec![];
        let item = create_item_with_content(ItemContent::Login(login));

        assert_eq!(item.get_field("urls"), Some(Field::Text("".to_string())));
        assert_eq!(item.get_field("url"), Some(Field::Text("".to_string())));
    }

    #[test]
    fn test_single_url_in_login() {
        let mut login = create_login_item();
        login.urls = vec!["https://single.com".to_string()];
        let item = create_item_with_content(ItemContent::Login(login));

        assert_eq!(
            item.get_field("urls"),
            Some(Field::Text("https://single.com".to_string()))
        );
        assert_eq!(
            item.get_field("url"),
            Some(Field::Text("https://single.com".to_string()))
        );
    }

    #[test]
    fn test_card_type_formatting() {
        let mut cc = create_credit_card_item();
        cc.card_type = CardType::AmericanExpress;
        let item = create_item_with_content(ItemContent::CreditCard(cc));

        assert_eq!(
            item.get_field("card_type"),
            Some(Field::Text("AmericanExpress".to_string()))
        );
        assert_eq!(
            item.get_field("type"),
            Some(Field::Text("AmericanExpress".to_string()))
        );
    }

    #[test]
    fn test_wifi_security_formatting() {
        let mut wifi = create_wifi_item();
        wifi.security = WifiSecurity::WPA3;
        let item = create_item_with_content(ItemContent::Wifi(wifi));

        assert_eq!(
            item.get_field("security"),
            Some(Field::Text("WPA3".to_string()))
        );
    }
}
