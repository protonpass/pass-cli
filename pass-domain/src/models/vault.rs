use crate::ShareId;
use crate::protos::vault::vault_v1;
use anyhow::{Context, Result, anyhow};

#[derive(Clone, Debug, serde::Serialize)]
pub struct VaultId(pub(crate) String);
display_for_basic!(VaultId);

impl VaultId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct Vault {
    pub id: VaultId,
    pub share_id: ShareId,
    pub content: VaultData,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct VaultData {
    pub name: String,
    pub description: String,
    pub display_preferences: VaultDisplayPreferences,
}

impl VaultData {
    pub fn new(
        name: String,
        description: String,
        display_preferences: VaultDisplayPreferences,
    ) -> Result<Self> {
        if name.is_empty() || description.is_empty() {
            return Err(anyhow!("The vault name and description cannot be empty."));
        }

        Ok(Self {
            name,
            description,
            display_preferences,
        })
    }

    pub fn serialize(self) -> Result<Vec<u8>> {
        let as_proto = vault_v1::Vault::from(self);
        as_proto
            .to_vec()
            .context("Error serializing vault to proto")
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let as_proto =
            vault_v1::Vault::decode_from_slice(data).context("Error decoding Vault from proto")?;
        Ok(Self::from(as_proto))
    }
}

impl From<VaultData> for vault_v1::Vault {
    fn from(value: VaultData) -> Self {
        vault_v1::Vault {
            name: value.name,
            description: value.description,
            display: protobuf::MessageField::some(vault_v1::VaultDisplayPreferences::from(
                value.display_preferences,
            )),
            ..Default::default()
        }
    }
}

impl From<vault_v1::Vault> for VaultData {
    fn from(value: vault_v1::Vault) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_preferences: VaultDisplayPreferences::from(value.display.unwrap_or_default()),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct VaultDisplayPreferences {
    pub icon: VaultIcon,
    pub color: VaultColor,
}

impl From<VaultDisplayPreferences> for vault_v1::VaultDisplayPreferences {
    fn from(value: VaultDisplayPreferences) -> Self {
        vault_v1::VaultDisplayPreferences {
            icon: vault_v1::VaultIcon::from(value.icon).into(),
            color: vault_v1::VaultColor::from(value.color).into(),
            ..Default::default()
        }
    }
}

impl From<vault_v1::VaultDisplayPreferences> for VaultDisplayPreferences {
    fn from(value: vault_v1::VaultDisplayPreferences) -> Self {
        VaultDisplayPreferences {
            icon: VaultIcon::from(value.icon.enum_value_or_default()),
            color: VaultColor::from(value.color.enum_value_or_default()),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub enum VaultIcon {
    #[default]
    Icon1,
    Icon2,
    Icon3,
    Icon4,
    Icon5,
    Icon6,
    Icon7,
    Icon8,
    Icon9,
    Icon10,
    Icon11,
    Icon12,
    Icon13,
    Icon14,
    Icon15,
    Icon16,
    Icon17,
    Icon18,
    Icon19,
    Icon20,
    Icon21,
    Icon22,
    Icon23,
    Icon24,
    Icon25,
    Icon26,
    Icon27,
    Icon28,
    Icon29,
    Icon30,
}

impl From<VaultIcon> for vault_v1::VaultIcon {
    fn from(item: VaultIcon) -> Self {
        match item {
            VaultIcon::Icon1 => vault_v1::VaultIcon::ICON1,
            VaultIcon::Icon2 => vault_v1::VaultIcon::ICON2,
            VaultIcon::Icon3 => vault_v1::VaultIcon::ICON3,
            VaultIcon::Icon4 => vault_v1::VaultIcon::ICON4,
            VaultIcon::Icon5 => vault_v1::VaultIcon::ICON5,
            VaultIcon::Icon6 => vault_v1::VaultIcon::ICON6,
            VaultIcon::Icon7 => vault_v1::VaultIcon::ICON7,
            VaultIcon::Icon8 => vault_v1::VaultIcon::ICON8,
            VaultIcon::Icon9 => vault_v1::VaultIcon::ICON9,
            VaultIcon::Icon10 => vault_v1::VaultIcon::ICON10,
            VaultIcon::Icon11 => vault_v1::VaultIcon::ICON11,
            VaultIcon::Icon12 => vault_v1::VaultIcon::ICON12,
            VaultIcon::Icon13 => vault_v1::VaultIcon::ICON13,
            VaultIcon::Icon14 => vault_v1::VaultIcon::ICON14,
            VaultIcon::Icon15 => vault_v1::VaultIcon::ICON15,
            VaultIcon::Icon16 => vault_v1::VaultIcon::ICON16,
            VaultIcon::Icon17 => vault_v1::VaultIcon::ICON17,
            VaultIcon::Icon18 => vault_v1::VaultIcon::ICON18,
            VaultIcon::Icon19 => vault_v1::VaultIcon::ICON19,
            VaultIcon::Icon20 => vault_v1::VaultIcon::ICON20,
            VaultIcon::Icon21 => vault_v1::VaultIcon::ICON21,
            VaultIcon::Icon22 => vault_v1::VaultIcon::ICON22,
            VaultIcon::Icon23 => vault_v1::VaultIcon::ICON23,
            VaultIcon::Icon24 => vault_v1::VaultIcon::ICON24,
            VaultIcon::Icon25 => vault_v1::VaultIcon::ICON25,
            VaultIcon::Icon26 => vault_v1::VaultIcon::ICON26,
            VaultIcon::Icon27 => vault_v1::VaultIcon::ICON27,
            VaultIcon::Icon28 => vault_v1::VaultIcon::ICON28,
            VaultIcon::Icon29 => vault_v1::VaultIcon::ICON29,
            VaultIcon::Icon30 => vault_v1::VaultIcon::ICON30,
        }
    }
}

impl From<vault_v1::VaultIcon> for VaultIcon {
    fn from(item: vault_v1::VaultIcon) -> Self {
        match item {
            vault_v1::VaultIcon::ICON1 => VaultIcon::Icon1,
            vault_v1::VaultIcon::ICON2 => VaultIcon::Icon2,
            vault_v1::VaultIcon::ICON3 => VaultIcon::Icon3,
            vault_v1::VaultIcon::ICON4 => VaultIcon::Icon4,
            vault_v1::VaultIcon::ICON5 => VaultIcon::Icon5,
            vault_v1::VaultIcon::ICON6 => VaultIcon::Icon6,
            vault_v1::VaultIcon::ICON7 => VaultIcon::Icon7,
            vault_v1::VaultIcon::ICON8 => VaultIcon::Icon8,
            vault_v1::VaultIcon::ICON9 => VaultIcon::Icon9,
            vault_v1::VaultIcon::ICON10 => VaultIcon::Icon10,
            vault_v1::VaultIcon::ICON11 => VaultIcon::Icon11,
            vault_v1::VaultIcon::ICON12 => VaultIcon::Icon12,
            vault_v1::VaultIcon::ICON13 => VaultIcon::Icon13,
            vault_v1::VaultIcon::ICON14 => VaultIcon::Icon14,
            vault_v1::VaultIcon::ICON15 => VaultIcon::Icon15,
            vault_v1::VaultIcon::ICON16 => VaultIcon::Icon16,
            vault_v1::VaultIcon::ICON17 => VaultIcon::Icon17,
            vault_v1::VaultIcon::ICON18 => VaultIcon::Icon18,
            vault_v1::VaultIcon::ICON19 => VaultIcon::Icon19,
            vault_v1::VaultIcon::ICON20 => VaultIcon::Icon20,
            vault_v1::VaultIcon::ICON21 => VaultIcon::Icon21,
            vault_v1::VaultIcon::ICON22 => VaultIcon::Icon22,
            vault_v1::VaultIcon::ICON23 => VaultIcon::Icon23,
            vault_v1::VaultIcon::ICON24 => VaultIcon::Icon24,
            vault_v1::VaultIcon::ICON25 => VaultIcon::Icon25,
            vault_v1::VaultIcon::ICON26 => VaultIcon::Icon26,
            vault_v1::VaultIcon::ICON27 => VaultIcon::Icon27,
            vault_v1::VaultIcon::ICON28 => VaultIcon::Icon28,
            vault_v1::VaultIcon::ICON29 => VaultIcon::Icon29,
            vault_v1::VaultIcon::ICON30 => VaultIcon::Icon30,

            _ => VaultIcon::Icon1,
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub enum VaultColor {
    #[default]
    Color1,
    Color2,
    Color3,
    Color4,
    Color5,
    Color6,
    Color7,
    Color8,
    Color9,
    Color10,
}

impl From<VaultColor> for vault_v1::VaultColor {
    fn from(item: VaultColor) -> Self {
        match item {
            VaultColor::Color1 => vault_v1::VaultColor::COLOR1,
            VaultColor::Color2 => vault_v1::VaultColor::COLOR2,
            VaultColor::Color3 => vault_v1::VaultColor::COLOR3,
            VaultColor::Color4 => vault_v1::VaultColor::COLOR4,
            VaultColor::Color5 => vault_v1::VaultColor::COLOR5,
            VaultColor::Color6 => vault_v1::VaultColor::COLOR6,
            VaultColor::Color7 => vault_v1::VaultColor::COLOR7,
            VaultColor::Color8 => vault_v1::VaultColor::COLOR8,
            VaultColor::Color9 => vault_v1::VaultColor::COLOR9,
            VaultColor::Color10 => vault_v1::VaultColor::COLOR10,
        }
    }
}

impl From<vault_v1::VaultColor> for VaultColor {
    fn from(item: vault_v1::VaultColor) -> Self {
        match item {
            vault_v1::VaultColor::COLOR1 => VaultColor::Color1,
            vault_v1::VaultColor::COLOR2 => VaultColor::Color2,
            vault_v1::VaultColor::COLOR3 => VaultColor::Color3,
            vault_v1::VaultColor::COLOR4 => VaultColor::Color4,
            vault_v1::VaultColor::COLOR5 => VaultColor::Color5,
            vault_v1::VaultColor::COLOR6 => VaultColor::Color6,
            vault_v1::VaultColor::COLOR7 => VaultColor::Color7,
            vault_v1::VaultColor::COLOR8 => VaultColor::Color8,
            vault_v1::VaultColor::COLOR9 => VaultColor::Color9,
            vault_v1::VaultColor::COLOR10 => VaultColor::Color10,

            _ => VaultColor::Color1,
        }
    }
}
