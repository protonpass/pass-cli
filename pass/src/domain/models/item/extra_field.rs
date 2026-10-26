/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton Pass.
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

use crate::domain::{ItemData, ItemExtraFieldContent, UpdateFieldResult, update_field};
use anyhow::{Context, Result, anyhow, bail};
use pass_derive::sdk_export;
use proton_pass_common::totp::TOTP;
use std::str::FromStr;

#[sdk_export]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtraFieldKind {
    #[default]
    Text,
    Hidden,
    Totp,
}

impl FromStr for ExtraFieldKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "hidden" => Ok(Self::Hidden),
            "totp" => Ok(Self::Totp),
            _ => bail!("Invalid field type '{value}'. Valid types: text, hidden, totp"),
        }
    }
}

impl ExtraFieldKind {
    pub fn content(self, value: &str) -> Result<ItemExtraFieldContent> {
        Ok(match self {
            Self::Text => ItemExtraFieldContent::Text(value.to_string()),
            Self::Hidden => ItemExtraFieldContent::Hidden(value.to_string()),
            Self::Totp => {
                validate_totp(value)?;
                ItemExtraFieldContent::Totp(value.to_string())
            }
        })
    }
}

pub fn validate_totp(value: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    let totp = TOTP::from_uri(value)
        .map_err(|e| anyhow!("{e:?}"))
        .context("Invalid TOTP value. Expected an otpauth:// URI or a base32 secret")?;
    if totp.secret.trim().is_empty() {
        bail!("Invalid TOTP value. The TOTP secret cannot be empty");
    }
    totp.generate_token(0)
        .map_err(|e| anyhow!("{e:?}"))
        .context("Invalid TOTP value. Could not generate a code from the secret")?;
    Ok(())
}

pub fn update_field_with_kind(
    item: &mut ItemData,
    field_name: &str,
    field_value: &str,
    kind: ExtraFieldKind,
) -> Result<UpdateFieldResult> {
    let result = update_field(item, field_name, field_value)?;
    if result != UpdateFieldResult::CustomFieldCreated || kind == ExtraFieldKind::Text {
        return Ok(result);
    }
    match kind.content(field_value) {
        Ok(content) => {
            if let Some(created) = item.extra_fields.last_mut() {
                created.content = content;
            }
            Ok(result)
        }
        Err(e) => {
            item.extra_fields.pop();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ItemContent, NoteItem};

    const VALID_TOTP_URI: &str =
        "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";

    fn note_item() -> ItemData {
        ItemData::new(
            "Note".to_string(),
            "".to_string(),
            "uuid".to_string(),
            ItemContent::Note(NoteItem),
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn parses_kinds_case_insensitively() {
        assert_eq!(ExtraFieldKind::Text, "text".parse().unwrap());
        assert_eq!(ExtraFieldKind::Hidden, "Hidden".parse().unwrap());
        assert_eq!(ExtraFieldKind::Totp, " TOTP ".parse().unwrap());
        assert!("timestamp".parse::<ExtraFieldKind>().is_err());
        assert!("".parse::<ExtraFieldKind>().is_err());
    }

    #[test]
    fn validate_totp_accepts_uri_secret_and_empty() {
        assert!(validate_totp(VALID_TOTP_URI).is_ok());
        assert!(validate_totp("JBSWY3DPEHPK3PXP").is_ok());
        assert!(validate_totp("").is_ok());
    }

    #[test]
    fn validate_totp_rejects_invalid_uris() {
        assert!(validate_totp("otpauth://...").is_err());
        assert!(validate_totp("https://example.com/?secret=JBSWY3DPEHPK3PXP").is_err());
        assert!(validate_totp("otpauth://hotp/Example?secret=JBSWY3DPEHPK3PXP").is_err());
        assert!(validate_totp("otpauth://totp/Example?issuer=Example").is_err());
        assert!(validate_totp("otpauth://totp/Example?secret=").is_err());
        assert!(validate_totp("   ").is_err());
    }

    #[test]
    fn content_maps_kinds() {
        assert_eq!(
            ItemExtraFieldContent::Text("a".to_string()),
            ExtraFieldKind::Text.content("a").unwrap()
        );
        assert_eq!(
            ItemExtraFieldContent::Hidden("".to_string()),
            ExtraFieldKind::Hidden.content("").unwrap()
        );
        assert_eq!(
            ItemExtraFieldContent::Totp(VALID_TOTP_URI.to_string()),
            ExtraFieldKind::Totp.content(VALID_TOTP_URI).unwrap()
        );
        assert!(ExtraFieldKind::Totp.content("otpauth://...").is_err());
    }

    #[test]
    fn update_with_kind_creates_hidden_field() {
        let mut item = note_item();
        let result =
            update_field_with_kind(&mut item, "Recovery key", "", ExtraFieldKind::Hidden).unwrap();
        assert_eq!(UpdateFieldResult::CustomFieldCreated, result);
        assert_eq!(1, item.extra_fields.len());
        assert_eq!("Recovery key", item.extra_fields[0].name);
        assert_eq!(
            ItemExtraFieldContent::Hidden("".to_string()),
            item.extra_fields[0].content
        );
    }

    #[test]
    fn update_with_kind_creates_totp_field() {
        let mut item = note_item();
        update_field_with_kind(&mut item, "2FA", VALID_TOTP_URI, ExtraFieldKind::Totp).unwrap();
        assert_eq!(
            ItemExtraFieldContent::Totp(VALID_TOTP_URI.to_string()),
            item.extra_fields[0].content
        );
    }

    #[test]
    fn update_with_kind_rejects_invalid_totp_without_creating_field() {
        let mut item = note_item();
        assert!(
            update_field_with_kind(&mut item, "2FA", "otpauth://...", ExtraFieldKind::Totp)
                .is_err()
        );
        assert!(item.extra_fields.is_empty());
    }

    #[test]
    fn update_with_kind_keeps_existing_field_kind() {
        let mut item = note_item();
        update_field_with_kind(&mut item, "Recovery key", "old", ExtraFieldKind::Hidden).unwrap();
        let result =
            update_field_with_kind(&mut item, "Recovery key", "new", ExtraFieldKind::Text).unwrap();
        assert_eq!(UpdateFieldResult::FieldUpdated, result);
        assert_eq!(1, item.extra_fields.len());
        assert_eq!(
            ItemExtraFieldContent::Hidden("new".to_string()),
            item.extra_fields[0].content
        );
    }

    #[test]
    fn update_with_kind_does_not_validate_existing_fields_as_totp() {
        let mut item = note_item();
        let result =
            update_field_with_kind(&mut item, "title", "New title", ExtraFieldKind::Totp).unwrap();
        assert_eq!(UpdateFieldResult::FieldUpdated, result);
        assert_eq!("New title", item.title);
        assert!(item.extra_fields.is_empty());
    }
}
