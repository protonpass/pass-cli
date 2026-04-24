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

pub mod authenticator;
pub mod callbacks;
pub mod client_builder;
pub mod config;
pub mod error;
pub mod extra_password;
pub mod interactive_login;
pub mod os;
pub mod personal_access_token;
pub mod post_login;
pub mod storage;
pub mod store;
mod utils;
pub mod web_login;

pub use authenticator::Authenticator;
pub use callbacks::{AuthEventHandler, CredentialProvider};
pub use client_builder::ENVIRONMENT_ENV_VAR;
pub use config::{ClientConfig, DebugConfig, PostLoginConfig, ProxyConfig};
pub use error::AuthError;
pub use storage::SessionStorage;
pub use store::{PassSessionStore, SharedPassSessionStore};
