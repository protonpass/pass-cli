/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

use anyhow::Result;
use pass_domain::{
    CardType, CreditCardItem, CustomItem, CustomSection, IdentityItem, ItemContent, ItemData,
    ItemExtraField, ItemExtraFieldContent, LoginItem, NoteItem, Passkey, PasskeyCreationData,
    SshKeyItem, WifiItem, WifiSecurity,
};

fn create_login_item_data() -> ItemData {
    ItemData::new(
        "Login Item".to_string(),
        "Login note".to_string(),
        "uuid-login".to_string(),
        ItemContent::Login(LoginItem {
            email: "user@example.com".to_string(),
            username: "testuser".to_string(),
            password: "password123".to_string(),
            urls: vec!["https://example.com".to_string()],
            totp_uri: "".to_string(),
            passkeys: vec![],
        }),
        vec![],
    )
    .unwrap()
}

#[test]
fn test_perform_update_basic_fields() -> Result<()> {
    let original = create_login_item_data();
    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.title = "Updated Login Item".to_string();
    updated.note = "Updated note".to_string();

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    assert_eq!(result.title, updated.title);
    assert_eq!(result.note, updated.note);
    assert_eq!(result.item_uuid, original.item_uuid);

    Ok(())
}

#[test]
fn test_perform_update_login_content_fields() -> Result<()> {
    let original = create_login_item_data();
    let original_bytes = original.clone().serialize()?;

    let new_email = "newemail@example.com".to_string();
    let new_username = "newusername".to_string();
    let new_password = "newpassword".to_string();

    let mut updated = original.clone();
    if let ItemContent::Login(ref mut login) = updated.content {
        login.email = new_email.clone();
        login.username = new_username.clone();
        login.password = new_password.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Login(login) = result.content {
        assert_eq!(login.email, new_email);
        assert_eq!(login.username, new_username);
        assert_eq!(login.password, new_password);
    } else {
        panic!("Expected Login content");
    }

    Ok(())
}

#[test]
fn test_perform_update_login_urls() -> Result<()> {
    let original = create_login_item_data();
    let original_bytes = original.clone().serialize()?;

    let new_urls = vec![
        "https://new1.com".to_string(),
        "https://new2.com".to_string(),
    ];

    let mut updated = original.clone();
    if let ItemContent::Login(ref mut login) = updated.content {
        login.urls = new_urls.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Login(login) = result.content {
        assert_eq!(login.urls.len(), new_urls.len());
        assert_eq!(login.urls[0], new_urls[0]);
        assert_eq!(login.urls[1], new_urls[1]);
    } else {
        panic!("Expected Login content");
    }

    Ok(())
}

#[test]
fn test_perform_update_with_passkeys() -> Result<()> {
    let passkey_key_id = "key1".to_string();
    let passkey_domain = "example.com".to_string();
    let new_title = "Updated with passkey".to_string();

    let mut original = create_login_item_data();
    if let ItemContent::Login(ref mut login) = original.content {
        login.passkeys = vec![Passkey {
            key_id: passkey_key_id.clone(),
            content: vec![1, 2, 3],
            domain: passkey_domain.clone(),
            rp_id: "example.com".to_string(),
            rp_name: "Example".to_string(),
            user_name: "user".to_string(),
            user_display_name: "User Display".to_string(),
            user_id: vec![4, 5, 6],
            create_time: 123456,
            note: "passkey note".to_string(),
            credential_id: vec![7, 8, 9],
            user_handle: vec![10, 11, 12],
            creation_data: Some(PasskeyCreationData {
                os_name: "macOS".to_string(),
                os_version: "14.0".to_string(),
                device_name: "MacBook".to_string(),
                app_version: "1.0".to_string(),
            }),
        }];
    }

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.title = new_title.clone();

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    assert_eq!(result.title, new_title);
    if let ItemContent::Login(login) = result.content {
        assert_eq!(login.passkeys.len(), 1);
        assert_eq!(login.passkeys[0].key_id, passkey_key_id);
        assert_eq!(login.passkeys[0].domain, passkey_domain);
        assert!(login.passkeys[0].creation_data.is_some());
    } else {
        panic!("Expected Login content");
    }

    Ok(())
}

#[test]
fn test_perform_update_extra_fields() -> Result<()> {
    let field1_name = "field1".to_string();
    let field2_name = "field2".to_string();
    let field3_name = "field3".to_string();
    let original_value1 = ItemExtraFieldContent::Text("value1".to_string());
    let original_value2 = ItemExtraFieldContent::Hidden("secret".to_string());
    let updated_value1 = ItemExtraFieldContent::Text("updated_value1".to_string());
    let value3 = ItemExtraFieldContent::Text("value3".to_string());

    let mut original = create_login_item_data();
    original.extra_fields = vec![
        ItemExtraField {
            name: field1_name.clone(),
            content: original_value1.clone(),
        },
        ItemExtraField {
            name: field2_name.clone(),
            content: original_value2.clone(),
        },
    ];

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.extra_fields = vec![
        ItemExtraField {
            name: field1_name.clone(),
            content: updated_value1.clone(),
        },
        ItemExtraField {
            name: field3_name.clone(),
            content: value3.clone(),
        },
    ];

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    // Fields should be replaced, not duplicated
    assert_eq!(result.extra_fields.len(), 2);
    assert_eq!(result.extra_fields[0].name, field1_name);
    assert_eq!(result.extra_fields[0].content, updated_value1);
    assert_eq!(result.extra_fields[1].name, field3_name);
    assert_eq!(result.extra_fields[1].content, value3);

    Ok(())
}

#[test]
fn test_perform_update_credit_card() -> Result<()> {
    let original_cardholder = "John Doe".to_string();
    let original_expiration = "12/25".to_string();
    let original_pin = "1234".to_string();
    let new_cardholder = "Jane Doe".to_string();
    let new_number = "5555555555554444".to_string();
    let new_cvv = "456".to_string();

    let original = ItemData::new(
        "Credit Card".to_string(),
        "Card note".to_string(),
        "uuid-cc".to_string(),
        ItemContent::CreditCard(CreditCardItem {
            cardholder_name: original_cardholder.clone(),
            card_type: CardType::Visa,
            number: "4111111111111111".to_string(),
            verification_number: "123".to_string(),
            expiration_date: original_expiration.clone(),
            pin: original_pin.clone(),
        }),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::CreditCard(ref mut cc) = updated.content {
        cc.cardholder_name = new_cardholder.clone();
        cc.number = new_number.clone();
        cc.verification_number = new_cvv.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::CreditCard(cc) = result.content {
        assert_eq!(cc.cardholder_name, new_cardholder);
        assert_eq!(cc.number, new_number);
        assert_eq!(cc.verification_number, new_cvv);
        assert_eq!(cc.expiration_date, original_expiration);
        assert_eq!(cc.pin, original_pin);
    } else {
        panic!("Expected CreditCard content");
    }

    Ok(())
}

#[test]
fn test_perform_update_identity() -> Result<()> {
    let original_phone = "+1234567890".to_string();
    let original_zip = "12345".to_string();
    let original_x_handle = "@johndoe".to_string();
    let new_full_name = "Jane Doe".to_string();
    let new_email = "jane@example.com".to_string();
    let new_street = "456 Oak Ave".to_string();
    let new_city = "Portland".to_string();
    let new_state = "OR".to_string();

    let original = ItemData::new(
        "Identity".to_string(),
        "Identity note".to_string(),
        "uuid-identity".to_string(),
        ItemContent::Identity(Box::new(IdentityItem {
            full_name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            phone_number: original_phone.clone(),
            first_name: "John".to_string(),
            middle_name: "Q".to_string(),
            last_name: "Doe".to_string(),
            birthdate: "1990-01-01".to_string(),
            gender: "Male".to_string(),
            extra_personal_details: vec![],
            organization: "ACME Corp".to_string(),
            street_address: "123 Main St".to_string(),
            zip_or_postal_code: original_zip.clone(),
            city: "Springfield".to_string(),
            state_or_province: "IL".to_string(),
            country_or_region: "USA".to_string(),
            floor: "2".to_string(),
            county: "Sangamon".to_string(),
            extra_address_details: vec![],
            social_security_number: "123-45-6789".to_string(),
            passport_number: "A12345678".to_string(),
            license_number: "D1234567".to_string(),
            website: "https://johndoe.com".to_string(),
            x_handle: original_x_handle.clone(),
            second_phone_number: "+0987654321".to_string(),
            linkedin: "johndoe".to_string(),
            reddit: "johndoe".to_string(),
            facebook: "johndoe".to_string(),
            yahoo: "johndoe".to_string(),
            instagram: "johndoe".to_string(),
            extra_contact_details: vec![],
            company: "ACME Corp".to_string(),
            job_title: "Engineer".to_string(),
            personal_website: "https://personal.johndoe.com".to_string(),
            work_phone_number: "+1111111111".to_string(),
            work_email: "john.doe@acme.com".to_string(),
            extra_work_details: vec![],
            extra_sections: vec![],
        })),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::Identity(ref mut identity) = updated.content {
        identity.full_name = new_full_name.clone();
        identity.email = new_email.clone();
        identity.street_address = new_street.clone();
        identity.city = new_city.clone();
        identity.state_or_province = new_state.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Identity(identity) = result.content {
        assert_eq!(identity.full_name, new_full_name);
        assert_eq!(identity.email, new_email);
        assert_eq!(identity.street_address, new_street);
        assert_eq!(identity.city, new_city);
        assert_eq!(identity.state_or_province, new_state);
        assert_eq!(identity.phone_number, original_phone);
        assert_eq!(identity.zip_or_postal_code, original_zip);
        assert_eq!(identity.x_handle, original_x_handle);
    } else {
        panic!("Expected Identity content");
    }

    Ok(())
}

#[test]
fn test_perform_update_ssh_key() -> Result<()> {
    let new_private_key = "-----BEGIN PRIVATE KEY-----\nNEW\n-----END PRIVATE KEY-----".to_string();
    let new_public_key = "ssh-rsa NEWKEY".to_string();

    let original = ItemData::new(
        "SSH Key".to_string(),
        "SSH note".to_string(),
        "uuid-ssh".to_string(),
        ItemContent::SshKey(SshKeyItem {
            private_key: "-----BEGIN PRIVATE KEY-----\nOLD\n-----END PRIVATE KEY-----".to_string(),
            public_key: "ssh-rsa OLDKEY".to_string(),
            sections: vec![],
        }),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::SshKey(ref mut ssh) = updated.content {
        ssh.private_key = new_private_key.clone();
        ssh.public_key = new_public_key.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::SshKey(ssh) = result.content {
        assert_eq!(ssh.private_key, new_private_key);
        assert_eq!(ssh.public_key, new_public_key);
    } else {
        panic!("Expected SshKey content");
    }

    Ok(())
}

#[test]
fn test_perform_update_wifi() -> Result<()> {
    let new_ssid = "NewNetwork".to_string();
    let new_password = "newpassword".to_string();
    let new_security = WifiSecurity::WPA3;

    let original = ItemData::new(
        "WiFi".to_string(),
        "WiFi note".to_string(),
        "uuid-wifi".to_string(),
        ItemContent::Wifi(WifiItem {
            ssid: "OldNetwork".to_string(),
            password: "oldpassword".to_string(),
            security: WifiSecurity::WPA2,
            sections: vec![],
        }),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::Wifi(ref mut wifi) = updated.content {
        wifi.ssid = new_ssid.clone();
        wifi.password = new_password.clone();
        wifi.security = new_security.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Wifi(wifi) = result.content {
        assert_eq!(wifi.ssid, new_ssid);
        assert_eq!(wifi.password, new_password);
        assert_eq!(wifi.security, new_security);
    } else {
        panic!("Expected Wifi content");
    }

    Ok(())
}

#[test]
fn test_perform_update_custom_item() -> Result<()> {
    let section_name = "Section 1".to_string();
    let field_name = "field1".to_string();
    let updated_field_value = ItemExtraFieldContent::Text("updated_value1".to_string());

    let original = ItemData::new(
        "Custom".to_string(),
        "Custom note".to_string(),
        "uuid-custom".to_string(),
        ItemContent::Custom(CustomItem {
            sections: vec![CustomSection {
                section_name: section_name.clone(),
                section_fields: vec![
                    ItemExtraField {
                        name: field_name.clone(),
                        content: ItemExtraFieldContent::Text("value1".to_string()),
                    },
                    ItemExtraField {
                        name: "field2".to_string(),
                        content: ItemExtraFieldContent::Hidden("secret".to_string()),
                    },
                ],
            }],
        }),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::Custom(ref mut custom) = updated.content {
        custom.sections = vec![CustomSection {
            section_name: section_name.clone(),
            section_fields: vec![ItemExtraField {
                name: field_name.clone(),
                content: updated_field_value.clone(),
            }],
        }];
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Custom(custom) = result.content {
        assert_eq!(custom.sections.len(), 1);
        assert_eq!(custom.sections[0].section_fields.len(), 1);
        assert_eq!(
            custom.sections[0].section_fields[0].content,
            updated_field_value
        );
    } else {
        panic!("Expected Custom content");
    }

    Ok(())
}

#[test]
fn test_perform_update_note_item() -> Result<()> {
    let new_title = "Updated Note".to_string();
    let new_note = "Updated note content".to_string();

    let original = ItemData::new(
        "Note".to_string(),
        "Original note content".to_string(),
        "uuid-note".to_string(),
        ItemContent::Note(NoteItem),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.title = new_title.clone();
    updated.note = new_note.clone();

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    assert_eq!(result.title, new_title);
    assert_eq!(result.note, new_note);

    Ok(())
}

#[test]
fn test_perform_update_multiple_times() -> Result<()> {
    let original = create_login_item_data();
    let mut current_bytes = original.clone().serialize()?;

    for i in 1..=5 {
        let mut updated = ItemData::deserialize(&current_bytes)?;
        let expected_title = format!("Update {}", i);
        updated.title = expected_title.clone();

        current_bytes = ItemData::perform_update(&current_bytes, &updated)?;

        let result = ItemData::deserialize(&current_bytes)?;
        assert_eq!(result.title, expected_title);
    }

    Ok(())
}

#[test]
fn test_perform_update_preserves_item_uuid() -> Result<()> {
    let new_uuid = "changed-uuid".to_string();

    let original = create_login_item_data();
    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.title = "Changed Title".to_string();
    updated.item_uuid = new_uuid.clone();

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    assert_eq!(result.item_uuid, new_uuid);

    Ok(())
}

#[test]
fn test_perform_update_with_timestamp_extra_field() -> Result<()> {
    let timestamp_value = 1609459200;
    let new_title = "Updated with timestamp".to_string();

    let mut original = create_login_item_data();
    original.extra_fields = vec![ItemExtraField {
        name: "timestamp_field".to_string(),
        content: ItemExtraFieldContent::Timestamp(timestamp_value),
    }];

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.title = new_title.clone();

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    assert_eq!(result.title, new_title);
    // Fields should not be duplicated
    assert_eq!(result.extra_fields.len(), 1);
    assert_eq!(
        result.extra_fields[0].content,
        ItemExtraFieldContent::Timestamp(timestamp_value)
    );

    Ok(())
}

#[test]
fn test_perform_update_with_totp_extra_field() -> Result<()> {
    let totp_uri = "otpauth://totp/test".to_string();
    let new_title = "Updated with TOTP".to_string();

    let mut original = create_login_item_data();
    original.extra_fields = vec![ItemExtraField {
        name: "totp".to_string(),
        content: ItemExtraFieldContent::Totp(totp_uri.clone()),
    }];

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    updated.title = new_title.clone();

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    assert_eq!(result.title, new_title);
    // Fields should not be duplicated
    assert_eq!(result.extra_fields.len(), 1);
    assert_eq!(
        result.extra_fields[0].content,
        ItemExtraFieldContent::Totp(totp_uri)
    );

    Ok(())
}

#[test]
fn test_perform_update_empty_to_populated() -> Result<()> {
    let new_email = "new@example.com".to_string();
    let new_username = "newuser".to_string();
    let new_password = "newpass".to_string();
    let new_url = "https://example.com".to_string();

    let original = ItemData::new(
        "Empty Login".to_string(),
        "".to_string(),
        "uuid-empty".to_string(),
        ItemContent::Login(LoginItem {
            email: "".to_string(),
            username: "".to_string(),
            password: "".to_string(),
            urls: vec![],
            totp_uri: "".to_string(),
            passkeys: vec![],
        }),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::Login(ref mut login) = updated.content {
        login.email = new_email.clone();
        login.username = new_username.clone();
        login.password = new_password.clone();
        login.urls = vec![new_url.clone()];
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Login(login) = result.content {
        assert_eq!(login.email, new_email);
        assert_eq!(login.username, new_username);
        assert_eq!(login.password, new_password);
        assert_eq!(login.urls.len(), 1);
        assert_eq!(login.urls[0], new_url);
    } else {
        panic!("Expected Login content");
    }

    Ok(())
}

#[test]
fn test_perform_update_identity_with_extra_sections() -> Result<()> {
    let original_full_name = "John Doe".to_string();
    let new_full_name = "Jane Doe".to_string();
    let section_name = "Custom Section".to_string();

    let original = ItemData::new(
        "Identity with sections".to_string(),
        "".to_string(),
        "uuid-identity-sections".to_string(),
        ItemContent::Identity(Box::new(IdentityItem {
            full_name: original_full_name,
            email: "john@example.com".to_string(),
            phone_number: "".to_string(),
            first_name: "".to_string(),
            middle_name: "".to_string(),
            last_name: "".to_string(),
            birthdate: "".to_string(),
            gender: "".to_string(),
            extra_personal_details: vec![],
            organization: "".to_string(),
            street_address: "".to_string(),
            zip_or_postal_code: "".to_string(),
            city: "".to_string(),
            state_or_province: "".to_string(),
            country_or_region: "".to_string(),
            floor: "".to_string(),
            county: "".to_string(),
            extra_address_details: vec![],
            social_security_number: "".to_string(),
            passport_number: "".to_string(),
            license_number: "".to_string(),
            website: "".to_string(),
            x_handle: "".to_string(),
            second_phone_number: "".to_string(),
            linkedin: "".to_string(),
            reddit: "".to_string(),
            facebook: "".to_string(),
            yahoo: "".to_string(),
            instagram: "".to_string(),
            extra_contact_details: vec![],
            company: "".to_string(),
            job_title: "".to_string(),
            personal_website: "".to_string(),
            work_phone_number: "".to_string(),
            work_email: "".to_string(),
            extra_work_details: vec![],
            extra_sections: vec![CustomSection {
                section_name: section_name.clone(),
                section_fields: vec![ItemExtraField {
                    name: "custom_field".to_string(),
                    content: ItemExtraFieldContent::Text("custom_value".to_string()),
                }],
            }],
        })),
        vec![],
    )?;

    let original_bytes = original.clone().serialize()?;

    let mut updated = original.clone();
    if let ItemContent::Identity(ref mut identity) = updated.content {
        identity.full_name = new_full_name.clone();
    }

    let updated_bytes = ItemData::perform_update(&original_bytes, &updated)?;
    let result = ItemData::deserialize(&updated_bytes)?;

    if let ItemContent::Identity(identity) = result.content {
        assert_eq!(identity.full_name, new_full_name);
        assert_eq!(identity.extra_sections.len(), 1);
        assert_eq!(identity.extra_sections[0].section_name, section_name);
    } else {
        panic!("Expected Identity content");
    }

    Ok(())
}
