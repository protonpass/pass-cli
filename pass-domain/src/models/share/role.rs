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

#[derive(Clone, Copy, Eq, PartialEq, Default, serde::Serialize)]
pub struct Permission(u16);

impl std::fmt::Debug for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Permission {{ value: {}, flags: {:?} }}",
            self.0,
            self.flags()
        )
    }
}

impl Permission {
    pub fn new(flags: Vec<PermissionFlag>) -> Self {
        let mut value = Self::default();
        for flag in flags {
            value.add_flag(flag);
        }
        value
    }

    pub fn new_from_value(value: u16) -> Self {
        Self(value)
    }

    pub fn new_from_role(role_id: &str, is_owner: bool, permission: u16) -> Permission {
        ShareRole::from_value(role_id, is_owner, permission).to_permission()
    }

    pub fn full() -> Self {
        Self::new(PermissionFlag::full_access())
    }

    pub fn add_flag(&mut self, flag: PermissionFlag) {
        self.0 |= flag.value();
    }

    pub fn has_flag(&self, flag: PermissionFlag) -> bool {
        let flag_value = flag.value();
        (self.0 & flag_value) == flag_value
    }

    pub fn flags(&self) -> Vec<PermissionFlag> {
        PermissionFlag::all()
            .into_iter()
            .filter(|f| self.has_flag(*f))
            .collect()
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionFlag {
    Admin,
    Read,
    Create,
    Update,
    Trash,
    Delete,
}

impl PermissionFlag {
    pub fn all() -> Vec<Self> {
        vec![
            PermissionFlag::Admin,
            PermissionFlag::Read,
            PermissionFlag::Create,
            PermissionFlag::Update,
            PermissionFlag::Trash,
            PermissionFlag::Delete,
        ]
    }

    pub fn value(&self) -> u16 {
        match self {
            PermissionFlag::Admin => 1 << 0,
            PermissionFlag::Read => 1 << 1,
            PermissionFlag::Create => 1 << 2,
            PermissionFlag::Update => 1 << 3,
            PermissionFlag::Trash => 1 << 4,
            PermissionFlag::Delete => 1 << 5,
        }
    }

    pub fn full_access() -> Vec<Self> {
        vec![
            PermissionFlag::Read,
            PermissionFlag::Create,
            PermissionFlag::Update,
            PermissionFlag::Trash,
            PermissionFlag::Delete,
        ]
    }

    pub fn name(&self) -> String {
        match self {
            PermissionFlag::Admin => "Admin",
            PermissionFlag::Read => "Read",
            PermissionFlag::Create => "Create",
            PermissionFlag::Update => "Update",
            PermissionFlag::Trash => "Trash",
            PermissionFlag::Delete => "Delete",
        }
        .to_string()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub enum ShareRole {
    Owner,
    Manager,
    Editor,
    Viewer,
    Custom {
        name: String,
        permission: Permission,
    },
}

impl std::fmt::Display for ShareRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl ShareRole {
    pub fn from_value(value: &str, is_owner: bool, permission: u16) -> Self {
        if is_owner {
            return ShareRole::Owner;
        }
        match value {
            "1" => ShareRole::Manager,
            "2" => ShareRole::Editor,
            "3" => ShareRole::Viewer,
            _ => ShareRole::Custom {
                name: value.to_string(),
                permission: Permission::new_from_value(permission),
            },
        }
    }

    pub fn to_permission(self) -> Permission {
        match self {
            ShareRole::Owner => Permission::new(vec![PermissionFlag::Admin]),
            ShareRole::Manager => Permission::new(vec![PermissionFlag::Admin]),
            ShareRole::Viewer => Permission::new(vec![PermissionFlag::Read]),
            ShareRole::Editor => Permission::new(PermissionFlag::full_access()),
            ShareRole::Custom {
                name: _,
                permission: p,
            } => p,
        }
    }

    pub fn value(&self) -> String {
        match self {
            ShareRole::Owner => "1",
            ShareRole::Manager => "1",
            ShareRole::Editor => "2",
            ShareRole::Viewer => "3",
            ShareRole::Custom { name, .. } => name,
        }
        .to_string()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn general_behaviour() {
        let mut invite_permission = Permission::default();
        assert_false!(invite_permission.has_flag(PermissionFlag::Read));

        invite_permission.add_flag(PermissionFlag::Read);
        assert_true!(invite_permission.has_flag(PermissionFlag::Read));
        assert_eq!(invite_permission.value(), PermissionFlag::Read.value());

        invite_permission.add_flag(PermissionFlag::Create);
        assert_true!(invite_permission.has_flag(PermissionFlag::Read));
        assert_true!(invite_permission.has_flag(PermissionFlag::Create));

        assert_eq!(
            invite_permission.value(),
            PermissionFlag::Create.value() | PermissionFlag::Read.value()
        )
    }

    #[test]
    fn idempotent() {
        let mut invite_permission = Permission::default();
        invite_permission.add_flag(PermissionFlag::Read);
        invite_permission.add_flag(PermissionFlag::Read);
        assert_true!(invite_permission.has_flag(PermissionFlag::Read));
        assert_eq!(invite_permission.value(), PermissionFlag::Read.value());
    }

    #[test]
    fn new_from_value() {
        let value = PermissionFlag::Admin.value() | PermissionFlag::Create.value();
        let permission = Permission::new_from_value(value);
        assert_true!(permission.has_flag(PermissionFlag::Admin));
        assert_true!(permission.has_flag(PermissionFlag::Create));
    }
}
