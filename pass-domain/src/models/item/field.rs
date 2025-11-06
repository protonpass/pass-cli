use crate::{CreditCardItem, CustomItem, IdentityItem, LoginItem, SshKeyItem, WifiItem};
use crate::{Item, ItemContent, ItemExtraField, ItemExtraFieldContent};

impl ItemExtraField {
    pub fn value(&self) -> String {
        match &self.content {
            ItemExtraFieldContent::Text(text) => text.to_string(),
            ItemExtraFieldContent::Totp(totp) => totp.to_string(),
            ItemExtraFieldContent::Hidden(value) => value.to_string(),
            ItemExtraFieldContent::Timestamp(timestamp) => format!("{timestamp}"),
        }
    }
}

impl Item {
    pub fn get_field(&self, field: &str) -> Option<String> {
        match field {
            "title" => Some(self.content.title.clone()),
            "note" => Some(self.content.note.clone()),
            _ => self.try_find_field(field),
        }
    }

    fn try_find_field(&self, field: &str) -> Option<String> {
        for extra_field in self.content.extra_fields.iter() {
            if extra_field.name.to_lowercase() == field.to_lowercase() {
                return Some(extra_field.value());
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

    fn find_login_field(&self, login: &LoginItem, field: &str) -> Option<String> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "email" => Some(login.email.clone()),
            "username" => Some(login.username.clone()),
            "password" => Some(login.password.clone()),
            "totp_uri" | "totp" => Some(login.totp_uri.clone()),
            "urls" | "url" => {
                if login.urls.is_empty() {
                    Some(String::new())
                } else {
                    Some(login.urls.join(", "))
                }
            }
            _ => None,
        }
    }

    fn find_cc_field(&self, cc: &CreditCardItem, field: &str) -> Option<String> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "cardholder_name" | "cardholder" | "name" => Some(cc.cardholder_name.clone()),
            "card_type" | "type" => Some(format!("{:?}", cc.card_type)),
            "number" | "card_number" => Some(cc.number.clone()),
            "verification_number" | "cvv" | "cvc" => Some(cc.verification_number.clone()),
            "expiration_date" | "expiry" | "exp_date" => Some(cc.expiration_date.clone()),
            "pin" => Some(cc.pin.clone()),
            _ => None,
        }
    }

    fn find_identity_field(&self, identity: &IdentityItem, field: &str) -> Option<String> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "full_name" | "fullname" => Some(identity.full_name.clone()),
            "email" => Some(identity.email.clone()),
            "phone_number" | "phone" => Some(identity.phone_number.clone()),
            "first_name" | "firstname" => Some(identity.first_name.clone()),
            "middle_name" | "middlename" => Some(identity.middle_name.clone()),
            "last_name" | "lastname" => Some(identity.last_name.clone()),
            "birthdate" | "birth_date" | "dob" => Some(identity.birthdate.clone()),
            "gender" => Some(identity.gender.clone()),
            "organization" | "org" => Some(identity.organization.clone()),
            "street_address" | "address" => Some(identity.street_address.clone()),
            "zip_or_postal_code" | "zip" | "postal_code" => {
                Some(identity.zip_or_postal_code.clone())
            }
            "city" => Some(identity.city.clone()),
            "state_or_province" | "state" | "province" => Some(identity.state_or_province.clone()),
            "country_or_region" | "country" | "region" => Some(identity.country_or_region.clone()),
            "social_security_number" | "ssn" => Some(identity.social_security_number.clone()),
            "passport_number" | "passport" => Some(identity.passport_number.clone()),
            "license_number" | "license" => Some(identity.license_number.clone()),
            "website" | "url" => Some(identity.website.clone()),
            "company" => Some(identity.company.clone()),
            "job_title" | "title" => Some(identity.job_title.clone()),
            _ => None,
        }
    }

    fn find_ssh_field(&self, ssh: &SshKeyItem, field: &str) -> Option<String> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "private_key" | "private" => Some(ssh.private_key.clone()),
            "public_key" | "public" => Some(ssh.public_key.clone()),
            _ => None,
        }
    }

    fn find_wifi_field(&self, wifi: &WifiItem, field: &str) -> Option<String> {
        let field_lower = field.to_lowercase();
        match field_lower.as_str() {
            "ssid" => Some(wifi.ssid.clone()),
            "password" => Some(wifi.password.clone()),
            "security" => Some(format!("{:?}", wifi.security)),
            _ => None,
        }
    }

    fn find_custom_field(&self, custom: &CustomItem, field: &str) -> Option<String> {
        for section in &custom.sections {
            for section_field in &section.section_fields {
                if section_field.name.to_lowercase() == field.to_lowercase() {
                    return Some(section_field.value());
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

        assert_eq!(item.get_field("title"), Some("Test Item".to_string()));
        assert_eq!(item.get_field("note"), Some("Test note".to_string()));
    }

    #[test]
    fn test_get_field_extra_fields() {
        let item = create_item_with_content(ItemContent::Note(NoteItem));

        assert_eq!(
            item.get_field("extra_field"),
            Some("extra_value".to_string())
        );
        assert_eq!(
            item.get_field("EXTRA_FIELD"),
            Some("extra_value".to_string())
        );
        assert_eq!(item.get_field("nonexistent"), None);
    }

    #[test]
    fn test_login_item_fields() {
        let item = create_item_with_content(ItemContent::Login(create_login_item()));

        assert_eq!(
            item.get_field("email"),
            Some("test@example.com".to_string())
        );
        assert_eq!(
            item.get_field("EMAIL"),
            Some("test@example.com".to_string())
        );
        assert_eq!(item.get_field("username"), Some("testuser".to_string()));
        assert_eq!(item.get_field("password"), Some("secretpass".to_string()));
        assert_eq!(
            item.get_field("totp_uri"),
            Some("otpauth://totp/test".to_string())
        );
        assert_eq!(
            item.get_field("totp"),
            Some("otpauth://totp/test".to_string())
        );
        assert_eq!(
            item.get_field("urls"),
            Some("https://example.com, https://test.com".to_string())
        );
        assert_eq!(
            item.get_field("url"),
            Some("https://example.com, https://test.com".to_string())
        );
    }

    #[test]
    fn test_credit_card_item_fields() {
        let item = create_item_with_content(ItemContent::CreditCard(create_credit_card_item()));

        assert_eq!(
            item.get_field("cardholder_name"),
            Some("John Doe".to_string())
        );
        assert_eq!(item.get_field("cardholder"), Some("John Doe".to_string()));
        assert_eq!(item.get_field("name"), Some("John Doe".to_string()));
        assert_eq!(item.get_field("card_type"), Some("Visa".to_string()));
        assert_eq!(item.get_field("type"), Some("Visa".to_string()));
        assert_eq!(
            item.get_field("number"),
            Some("4111111111111111".to_string())
        );
        assert_eq!(
            item.get_field("card_number"),
            Some("4111111111111111".to_string())
        );
        assert_eq!(
            item.get_field("verification_number"),
            Some("123".to_string())
        );
        assert_eq!(item.get_field("cvv"), Some("123".to_string()));
        assert_eq!(item.get_field("cvc"), Some("123".to_string()));
        assert_eq!(item.get_field("expiration_date"), Some("12/25".to_string()));
        assert_eq!(item.get_field("expiry"), Some("12/25".to_string()));
        assert_eq!(item.get_field("exp_date"), Some("12/25".to_string()));
        assert_eq!(item.get_field("pin"), Some("1234".to_string()));
    }

    #[test]
    fn test_identity_item_fields() {
        let item = create_item_with_content(ItemContent::Identity(create_identity_item()));

        assert_eq!(item.get_field("full_name"), Some("John Doe".to_string()));
        assert_eq!(item.get_field("fullname"), Some("John Doe".to_string()));
        assert_eq!(
            item.get_field("email"),
            Some("john@example.com".to_string())
        );
        assert_eq!(
            item.get_field("phone_number"),
            Some("+1234567890".to_string())
        );
        assert_eq!(item.get_field("phone"), Some("+1234567890".to_string()));
        assert_eq!(item.get_field("first_name"), Some("John".to_string()));
        assert_eq!(item.get_field("firstname"), Some("John".to_string()));
        assert_eq!(item.get_field("middle_name"), Some("Michael".to_string()));
        assert_eq!(item.get_field("middlename"), Some("Michael".to_string()));
        assert_eq!(item.get_field("last_name"), Some("Doe".to_string()));
        assert_eq!(item.get_field("lastname"), Some("Doe".to_string()));
        assert_eq!(item.get_field("birthdate"), Some("1990-01-01".to_string()));
        assert_eq!(item.get_field("birth_date"), Some("1990-01-01".to_string()));
        assert_eq!(item.get_field("dob"), Some("1990-01-01".to_string()));
        assert_eq!(item.get_field("gender"), Some("Male".to_string()));
        assert_eq!(
            item.get_field("organization"),
            Some("Test Corp".to_string())
        );
        assert_eq!(item.get_field("org"), Some("Test Corp".to_string()));
        assert_eq!(
            item.get_field("street_address"),
            Some("123 Main St".to_string())
        );
        assert_eq!(item.get_field("address"), Some("123 Main St".to_string()));
        assert_eq!(
            item.get_field("zip_or_postal_code"),
            Some("12345".to_string())
        );
        assert_eq!(item.get_field("zip"), Some("12345".to_string()));
        assert_eq!(item.get_field("postal_code"), Some("12345".to_string()));
        assert_eq!(item.get_field("city"), Some("Test City".to_string()));
        assert_eq!(
            item.get_field("state_or_province"),
            Some("Test State".to_string())
        );
        assert_eq!(item.get_field("state"), Some("Test State".to_string()));
        assert_eq!(item.get_field("province"), Some("Test State".to_string()));
        assert_eq!(
            item.get_field("country_or_region"),
            Some("Test Country".to_string())
        );
        assert_eq!(item.get_field("country"), Some("Test Country".to_string()));
        assert_eq!(item.get_field("region"), Some("Test Country".to_string()));
        assert_eq!(
            item.get_field("social_security_number"),
            Some("123-45-6789".to_string())
        );
        assert_eq!(item.get_field("ssn"), Some("123-45-6789".to_string()));
        assert_eq!(
            item.get_field("passport_number"),
            Some("A1234567".to_string())
        );
        assert_eq!(item.get_field("passport"), Some("A1234567".to_string()));
        assert_eq!(
            item.get_field("license_number"),
            Some("D123456789".to_string())
        );
        assert_eq!(item.get_field("license"), Some("D123456789".to_string()));
        assert_eq!(
            item.get_field("website"),
            Some("https://johndoe.com".to_string())
        );
        assert_eq!(
            item.get_field("url"),
            Some("https://johndoe.com".to_string())
        );
        assert_eq!(item.get_field("company"), Some("Acme Inc".to_string()));
        assert_eq!(
            item.get_field("job_title"),
            Some("Software Engineer".to_string())
        );
    }

    #[test]
    fn test_ssh_key_item_fields() {
        let item = create_item_with_content(ItemContent::SshKey(create_ssh_key_item()));

        assert_eq!(
            item.get_field("private_key"),
            Some("-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBg...".to_string())
        );
        assert_eq!(
            item.get_field("private"),
            Some("-----BEGIN PRIVATE KEY-----\nMIIEvgIBADANBg...".to_string())
        );
        assert_eq!(
            item.get_field("public_key"),
            Some("ssh-rsa AAAAB3NzaC1yc2E...".to_string())
        );
        assert_eq!(
            item.get_field("public"),
            Some("ssh-rsa AAAAB3NzaC1yc2E...".to_string())
        );
    }

    #[test]
    fn test_wifi_item_fields() {
        let item = create_item_with_content(ItemContent::Wifi(create_wifi_item()));

        assert_eq!(item.get_field("ssid"), Some("TestNetwork".to_string()));
        assert_eq!(item.get_field("password"), Some("wifipass123".to_string()));
        assert_eq!(item.get_field("security"), Some("WPA2".to_string()));
    }

    #[test]
    fn test_custom_item_fields() {
        let item = create_item_with_content(ItemContent::Custom(create_custom_item()));

        assert_eq!(item.get_field("custom_field1"), Some("value1".to_string()));
        assert_eq!(item.get_field("CUSTOM_FIELD1"), Some("value1".to_string()));
        assert_eq!(
            item.get_field("custom_field2"),
            Some("secret_value".to_string())
        );
        assert_eq!(item.get_field("nonexistent_field"), None);
    }

    #[test]
    fn test_note_and_alias_items() {
        let note_item = create_item_with_content(ItemContent::Note(NoteItem));
        let alias_item = create_item_with_content(ItemContent::Alias(AliasItem));

        // Should only find basic fields and extra fields
        assert_eq!(note_item.get_field("title"), Some("Test Item".to_string()));
        assert_eq!(note_item.get_field("note"), Some("Test note".to_string()));
        assert_eq!(
            note_item.get_field("extra_field"),
            Some("extra_value".to_string())
        );
        assert_eq!(note_item.get_field("nonexistent"), None);

        assert_eq!(alias_item.get_field("title"), Some("Test Item".to_string()));
        assert_eq!(alias_item.get_field("note"), Some("Test note".to_string()));
        assert_eq!(
            alias_item.get_field("extra_field"),
            Some("extra_value".to_string())
        );
        assert_eq!(alias_item.get_field("nonexistent"), None);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let item = create_item_with_content(ItemContent::Login(create_login_item()));

        // Test various case combinations
        assert_eq!(
            item.get_field("EMAIL"),
            Some("test@example.com".to_string())
        );
        assert_eq!(
            item.get_field("Email"),
            Some("test@example.com".to_string())
        );
        assert_eq!(
            item.get_field("eMaIl"),
            Some("test@example.com".to_string())
        );
        assert_eq!(item.get_field("USERNAME"), Some("testuser".to_string()));
        assert_eq!(item.get_field("Password"), Some("secretpass".to_string()));
        assert_eq!(
            item.get_field("TOTP_URI"),
            Some("otpauth://totp/test".to_string())
        );
    }

    #[test]
    fn test_empty_urls_in_login() {
        let mut login = create_login_item();
        login.urls = vec![];
        let item = create_item_with_content(ItemContent::Login(login));

        assert_eq!(item.get_field("urls"), Some("".to_string()));
        assert_eq!(item.get_field("url"), Some("".to_string()));
    }

    #[test]
    fn test_single_url_in_login() {
        let mut login = create_login_item();
        login.urls = vec!["https://single.com".to_string()];
        let item = create_item_with_content(ItemContent::Login(login));

        assert_eq!(
            item.get_field("urls"),
            Some("https://single.com".to_string())
        );
        assert_eq!(
            item.get_field("url"),
            Some("https://single.com".to_string())
        );
    }

    #[test]
    fn test_card_type_formatting() {
        let mut cc = create_credit_card_item();
        cc.card_type = CardType::AmericanExpress;
        let item = create_item_with_content(ItemContent::CreditCard(cc));

        assert_eq!(
            item.get_field("card_type"),
            Some("AmericanExpress".to_string())
        );
        assert_eq!(item.get_field("type"), Some("AmericanExpress".to_string()));
    }

    #[test]
    fn test_wifi_security_formatting() {
        let mut wifi = create_wifi_item();
        wifi.security = WifiSecurity::WPA3;
        let item = create_item_with_content(ItemContent::Wifi(wifi));

        assert_eq!(item.get_field("security"), Some("WPA3".to_string()));
    }
}
