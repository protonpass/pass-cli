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
#![recursion_limit = "256"]

#[macro_use]
extern crate tracing;

#[macro_use]
mod macros;

// Named explicitly as the word `pass` is a keyword in some languages.
#[cfg(uniffi_runtime)]
uniffi::setup_scaffolding!("pass_core");

#[cfg(test)]
#[macro_use]
mod test_tools;

pub mod auth;
pub mod domain;
pub mod pgp;

mod account;
mod cache;
mod cache_control;
mod client;
mod common;
mod constants;
mod core_events;
pub use core_events::bootstrap_event_sync;
mod crypto;
mod error;
mod events;
mod feature_flags;
mod first_time_setup;
mod folder;
mod info;
#[cfg(feature = "internal")]
pub mod internal;
mod invite;
mod item;
mod local_crypto;
mod logout;
pub mod monitor;
pub(crate) mod muon_ext;
mod organization;
mod pagination;
pub mod password;
mod permission;
pub(crate) mod personal_access_token;
mod ping;
mod reports;
#[cfg(any(wasm_runtime, uniffi_runtime))]
pub mod sdk;
mod share;
mod telemetry;
mod user;
mod user_keys;
mod utils;
mod vault;

pub use account::settings::AccountUserSettings;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub use auth::Authenticator;
pub use auth::store::{PassSessionStore, SharedPassSessionStore};
pub use auth::{
    AuthError, AuthEventHandler, ClientConfig, CredentialProvider, DebugConfig, PostLoginConfig,
    ProxyConfig, SessionStorage,
};
pub use client::{PassClient, PassClientContext, PassSessionKeyType};
pub use domain::{ClientFeatures, LocalKeyProvider};
pub use error::{AnyhowErrorExt, SessionInvalidatedError};
pub use first_time_setup::FirstTimeSetupKey;
pub use folder::create::CreateFolderPayload;
pub use item::create::{credit_card, custom, identity, login, note, ssh_key, wifi};
pub use item::find::FindItemQuery;
pub use organization::{
    OrganizationAliasCreateMode, OrganizationExportMode, OrganizationInfo,
    OrganizationPasswordPolicy, OrganizationSettings, OrganizationShareMode,
    OrganizationVaultCreateMode,
};
pub use personal_access_token::{
    CreatePersonalAccessTokenArgs, CreatePersonalAccessTokenResponse, PersonalAccessToken,
    PersonalAccessTokenAccess, RenewPersonalAccessTokenResponse,
};
pub use reports::{BreachCount, ItemsReport, MemberReport, MonitorReport, Report};
pub use user::access::{PassPlan, PlanType, UserDataSettings, UserInfo};
pub use user::lock::SessionLockState;
pub use utils::is_id;
pub use vault::{CreateVaultArgs, UpdateVaultArgs};

pub use muon;
pub use proton_pass_common;
pub use proton_pass_types;
