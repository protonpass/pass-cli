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

pub mod callbacks;
pub mod config;
pub mod error;
pub mod extra_password;
pub mod personal_access_token;
pub mod post_login;
pub mod storage;
pub mod store;
mod utils;
pub mod web_login;

pub use callbacks::{AuthEventHandler, CredentialProvider};
pub use config::{ClientConfig, DebugConfig, ExtraHeader, PostLoginConfig, ProxyConfig};
pub use error::AuthError;
pub use storage::SessionStorage;
pub use store::{PassSessionStore, SharedPassSessionStore};

#[cfg(any(tokio_runtime, wasm_runtime))]
pub mod authenticator;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub mod client_builder;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub mod cookie_store;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub mod os;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub mod runtime;

#[cfg(tokio_runtime)]
pub mod tokio_runtime;
#[cfg(wasm_runtime)]
pub mod wasm_runtime;

#[cfg(any(tokio_runtime, wasm_runtime))]
pub use authenticator::Authenticator;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub use cookie_store::PassCookieStore;
#[cfg(any(tokio_runtime, wasm_runtime))]
pub use runtime::AuthRuntime;
#[cfg(tokio_runtime)]
pub use tokio_runtime::{
    TokioClient, TokioContext, TokioExecutor, TokioOs, TokioRuntime, TokioSession,
};
#[cfg(wasm_runtime)]
pub use wasm_runtime::{
    MonotonicClock, WasmClient, WasmContext, WasmExecutor, WasmRuntime, WasmSession, WasmTime,
};
