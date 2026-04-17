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

#[macro_use]
extern crate tracing;

#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
mod test_tools;

mod account;
mod cache;
mod client;
mod common;
mod constants;
mod crypto;
mod error;
mod events;
mod feature_flags;
mod first_time_setup;
mod folder;
mod info;
mod invite;
mod item;
mod local_crypto;
mod logout;
pub mod monitor;
pub(crate) mod muon_ext;
mod pagination;
pub mod password;
mod permission;
pub(crate) mod personal_access_token;
mod ping;
mod share;
mod telemetry;
mod user;
mod user_keys;
mod utils;
mod vault;

pub use account::settings::AccountUserSettings;
pub use client::{PassClient, PassClientContext, PassSessionKeyType};
pub use error::{AnyhowErrorExt, SessionInvalidatedError};
pub use first_time_setup::FirstTimeSetupKey;
pub use folder::create::CreateFolderPayload;
pub use item::create::credit_card;
pub use item::create::custom;
pub use item::create::identity;
pub use item::create::login;
pub use item::create::note;
pub use item::create::ssh_key;
pub use item::create::wifi;
pub use item::find::FindItemQuery;
pub use personal_access_token::{
    CreatePersonalAccessTokenArgs, CreatePersonalAccessTokenResponse, PersonalAccessToken,
    PersonalAccessTokenAccess, RenewPersonalAccessTokenResponse,
};
pub use user::access::{PassPlan, PlanType, UserDataSettings, UserInfo};
pub use utils::is_id;
pub use vault::{CreateVaultArgs, UpdateVaultArgs};
