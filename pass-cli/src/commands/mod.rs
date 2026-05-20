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

use clap::ValueEnum;
use pass_domain::ShareRole;

pub mod agent;
pub mod info;
pub mod inject;
#[cfg(feature = "internal")]
pub mod internal;
pub mod invite;
pub mod item;
pub mod login;
pub mod login_pat;
pub mod logout;
pub mod password;
pub mod personal_access_token;
pub mod run;
pub mod secret_resolver;
pub mod settings;
pub mod settings_helper;
pub mod share;
pub mod ssh_agent;
pub mod support;
pub mod test;
pub mod totp;
pub mod update;
pub mod user;
pub mod vault;

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Role {
    Viewer,
    Editor,
    Manager,
}

impl From<Role> for ShareRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Viewer => ShareRole::Viewer,
            Role::Editor => ShareRole::Editor,
            Role::Manager => ShareRole::Manager,
        }
    }
}
