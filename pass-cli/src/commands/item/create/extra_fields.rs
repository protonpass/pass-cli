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

use anyhow::{Context, Result};
use pass::custom::{CustomFieldContentPayload, CustomFieldPayload};
use pass::domain::ExtraFieldKind;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ExtraFieldTemplate {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub value: String,
}

impl TryFrom<ExtraFieldTemplate> for CustomFieldPayload {
    type Error = anyhow::Error;

    fn try_from(value: ExtraFieldTemplate) -> Result<Self> {
        let kind: ExtraFieldKind = value
            .kind
            .parse()
            .with_context(|| format!("Invalid extra field '{}'", value.title))?;
        Ok(Self {
            field_name: value.title,
            content: CustomFieldContentPayload::new(kind, value.value),
        })
    }
}

pub fn into_payloads(fields: Vec<ExtraFieldTemplate>) -> Result<Vec<CustomFieldPayload>> {
    fields
        .into_iter()
        .map(CustomFieldPayload::try_from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_templates_to_payloads() {
        let fields: Vec<ExtraFieldTemplate> = serde_json::from_str(
            r#"[
                {"kind": "hidden", "title": "Recovery key"},
                {"kind": "Text", "title": "Note", "value": "hello"},
                {"kind": "totp", "title": "2FA", "value": "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP"}
            ]"#,
        )
        .unwrap();
        let payloads = into_payloads(fields).unwrap();

        assert_eq!(3, payloads.len());
        assert_eq!("Recovery key", payloads[0].field_name);
        assert!(matches!(
            &payloads[0].content,
            CustomFieldContentPayload::Hidden(v) if v.is_empty()
        ));
        assert_eq!("Note", payloads[1].field_name);
        assert!(matches!(
            &payloads[1].content,
            CustomFieldContentPayload::Text(v) if v == "hello"
        ));
        assert_eq!("2FA", payloads[2].field_name);
        assert!(matches!(
            &payloads[2].content,
            CustomFieldContentPayload::Totp(v) if v == "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP"
        ));
    }

    #[test]
    fn unknown_kind_fails() {
        let fields: Vec<ExtraFieldTemplate> =
            serde_json::from_str(r#"[{"kind": "secret", "title": "X"}]"#).unwrap();
        let msg = format!("{:#}", into_payloads(fields).unwrap_err());
        assert!(msg.contains("Invalid extra field 'X'"));
        assert!(msg.contains("Valid types: text, hidden, totp"));
    }

    #[test]
    fn timestamp_kind_is_not_supported() {
        let fields: Vec<ExtraFieldTemplate> =
            serde_json::from_str(r#"[{"kind": "timestamp", "title": "X", "value": "1"}]"#).unwrap();
        assert!(into_payloads(fields).is_err());
    }
}
