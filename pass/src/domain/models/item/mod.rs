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

mod attachment;
mod extra_field;
mod field;
pub use attachment::*;
pub use extra_field::*;

// The item domain models - including the proto wire format (`serialize`, `deserialize`,
// `perform_update`) - come from the upstream `proton-pass-types` crate (proton-pass-common repo).
// We deliberately do not duplicate them here; only the API-envelope types that the REST layer
// needs (`Item`) are defined locally.
//
// Serde derives on these types are gated behind the upstream `wasm` feature, which we enable (see
// workspace Cargo.toml): it activates `serde::Serialize`/`serde::Deserialize` via
// `proton_pass_derive::ffi_type` and generates no wasm-specific code on native targets.
pub use proton_pass_types::{
    AliasItem, AllowedAndroidApp, AndroidSpecific, AutofillUrl, AutofillUrlMode, CardType,
    CreditCardItem, CustomItem, CustomSection, Field, IdentityItem, ItemContent, ItemData,
    ItemExtraField, ItemExtraFieldContent, ItemFlag, ItemId, ItemState, LoginItem, NoteItem,
    Passkey, PasskeyCreationData, PlatformSpecific, SshKeyItem, UpdateFieldResult, WifiItem,
    WifiSecurity,
};

use crate::domain::{FolderId, ShareId, VaultId};
use anyhow::{Result, anyhow};
use pass_derive::sdk_export;

/// Update a single field of an [`ItemData`], extending upstream's `ItemData::update_field` with
/// behaviours this CLI ships but `proton-pass-types` (tag 2.1.0) does not yet implement:
///
/// - Updating a top-level `Timestamp` extra field with a Unix timestamp value (upstream rejects
///   these outright).
/// - Updating fields inside `Identity` extra detail lists and extra sections, `SshKey`/`Wifi`
///   sections, and `Custom` sections - including the section-qualified `"Section.field"` syntax.
///   Without this, upstream treats such names as unknown and appends a spurious top-level text
///   extra field instead. A nested field that matches but can't take the value (an invalid
///   timestamp, or a TOTP field) is an error, as it is for top-level fields.
///
/// Everything else (title/note, non-timestamp top-level extra fields, item-type scalar fields,
/// and creation of new text extra fields for unknown names) is delegated verbatim to the upstream
/// implementation. These extensions are candidates for upstreaming; remove them once available.
pub fn update_field(
    item: &mut ItemData,
    field_name: &str,
    field_value: &str,
) -> Result<UpdateFieldResult> {
    let field_name_lower = field_name.to_lowercase();

    // Shipped behaviour: a top-level Timestamp extra field is updated by parsing a Unix timestamp
    // in seconds. Upstream returns an error for these, so handle them before delegating.
    for extra_field in item.extra_fields.iter_mut() {
        if extra_field.name.to_lowercase() == field_name_lower {
            if let ItemExtraFieldContent::Timestamp(_) = extra_field.content {
                extra_field.content =
                    ItemExtraFieldContent::Timestamp(parse_timestamp(field_value)?);
                return Ok(UpdateFieldResult::FieldUpdated);
            }
            // Any other content kind: upstream handles it identically.
            break;
        }
    }

    // Delegate first so upstream's scalar-field matching keeps priority over nested container
    // fields (matching the historical order of this CLI's updater).
    let result = item.update_field(field_name, field_value)?;
    if result != UpdateFieldResult::CustomFieldCreated {
        return Ok(result);
    }

    // Upstream did not recognise the name and appended a new text extra field. Before accepting
    // that, check whether the name targets a field inside a nested container (identity extra
    // details/sections, ssh/wifi/custom sections). If so, undo the append and update in place;
    // if it matches a field that can't take the value, fail without creating the field.
    let created_field = item.extra_fields.pop();
    if update_nested_field(item, &field_name_lower, field_value)? {
        return Ok(UpdateFieldResult::FieldUpdated);
    }
    if let Some(field) = created_field {
        item.extra_fields.push(field);
    }
    Ok(result)
}

fn parse_timestamp(field_value: &str) -> Result<i64> {
    field_value.trim().parse().map_err(|_| {
        anyhow!(
            "Invalid timestamp value '{}'. Expected a Unix timestamp in seconds (e.g., 1783029600).",
            field_value
        )
    })
}

/// Try to update a field inside the nested containers upstream's updater ignores.
/// Returns `Ok(false)` if no nested field has that name, and an error if one does but can't take
/// the value. Field values keep their existing content kind: `Timestamp` values are parsed as
/// Unix timestamps, and `Totp` fields can't be edited (as for top-level fields).
fn update_nested_field(
    item: &mut ItemData,
    field_name_lower: &str,
    field_value: &str,
) -> Result<bool> {
    Ok(match &mut item.content {
        ItemContent::Identity(identity) => {
            update_extra_fields(
                &mut identity.extra_personal_details,
                field_name_lower,
                field_value,
            )? || update_extra_fields(
                &mut identity.extra_address_details,
                field_name_lower,
                field_value,
            )? || update_extra_fields(
                &mut identity.extra_contact_details,
                field_name_lower,
                field_value,
            )? || update_extra_fields(
                &mut identity.extra_work_details,
                field_name_lower,
                field_value,
            )? || update_section_fields(
                &mut identity.extra_sections,
                field_name_lower,
                field_value,
            )?
        }
        ItemContent::SshKey(ssh) => {
            update_section_fields(&mut ssh.sections, field_name_lower, field_value)?
        }
        ItemContent::Wifi(wifi) => {
            update_section_fields(&mut wifi.sections, field_name_lower, field_value)?
        }
        ItemContent::Custom(custom) => {
            update_section_fields(&mut custom.sections, field_name_lower, field_value)?
        }
        ItemContent::Note(_)
        | ItemContent::Login(_)
        | ItemContent::Alias(_)
        | ItemContent::CreditCard(_) => false,
    })
}

/// Update a field within a `[CustomSection]` using section-qualified (`"section.field"`) or
/// unqualified field name syntax. Returns `Ok(false)` if no field matches.
fn update_section_fields(
    sections: &mut [CustomSection],
    field_name_lower: &str,
    field_value: &str,
) -> Result<bool> {
    // Section-qualified lookup first ("section.field")
    if let Some((section_name, field_name_only)) = field_name_lower.split_once('.') {
        for section in sections.iter_mut() {
            if section.section_name.to_lowercase() == section_name {
                for field in &mut section.section_fields {
                    if field.name.to_lowercase() == field_name_only {
                        update_extra_field_value(field, field_value)?;
                        return Ok(true);
                    }
                }
            }
        }
        return Ok(false);
    }

    // Fall back to searching all sections by unqualified field name
    for section in sections.iter_mut() {
        for field in &mut section.section_fields {
            if field.name.to_lowercase() == field_name_lower {
                update_extra_field_value(field, field_value)?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Update a field within a slice of extra fields by (unqualified) field name. Returns
/// `Ok(false)` if no field matches.
fn update_extra_fields(
    fields: &mut [ItemExtraField],
    field_name_lower: &str,
    field_value: &str,
) -> Result<bool> {
    for field in fields.iter_mut() {
        if field.name.to_lowercase() == field_name_lower {
            update_extra_field_value(field, field_value)?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Update an extra field's value, preserving its content kind. Timestamp values are parsed as
/// Unix timestamps; TOTP fields can't be edited (same error as upstream for top-level ones).
fn update_extra_field_value(field: &mut ItemExtraField, field_value: &str) -> Result<()> {
    field.content = match &field.content {
        ItemExtraFieldContent::Hidden(_) => ItemExtraFieldContent::Hidden(field_value.to_string()),
        ItemExtraFieldContent::Text(_) => ItemExtraFieldContent::Text(field_value.to_string()),
        ItemExtraFieldContent::Totp(_) => {
            return Err(anyhow!("Editing TOTP fields is unsupported"));
        }
        ItemExtraFieldContent::Timestamp(_) => {
            ItemExtraFieldContent::Timestamp(parse_timestamp(field_value)?)
        }
    };
    Ok(())
}

#[sdk_export]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Item {
    pub id: ItemId,
    pub share_id: ShareId,
    pub vault_id: VaultId,
    pub content: ItemData,
    pub state: ItemState,
    pub flags: Vec<ItemFlag>,
    pub create_time: jiff::civil::DateTime,
    pub modify_time: jiff::civil::DateTime,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub folder_id: Option<FolderId>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias_email: Option<String>,
}
