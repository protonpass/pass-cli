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

use crate::auth::cookie_store::PassCookieStore;
use crate::auth::store::PassMuonSessionStore;
use muon::common::GenericContext;
use muon::cookie_store::SharedCookieStore;

use muon::{Client, NoInfo, Session};

#[cfg(tokio_runtime)]
pub type PassConnector<Os> = muon::transport::http::hyper::connector::HyperConnector<Os>;
#[cfg(wasm_runtime)]
pub type PassConnector<Sleeper> = muon::transport::http::reqwest::ReqwestConnector<Sleeper>;

#[cfg(tokio_runtime)]
pub type PassCookieHandler<Os> =
    SharedCookieStore<PassCookieStore, <PassConnector<Os> as muon::rt::HasTimeCapabilities>::Time>;
#[cfg(wasm_runtime)]
pub type PassCookieHandler<Sleeper> = SharedCookieStore<PassCookieStore, Sleeper>;

#[cfg(tokio_runtime)]
pub type Context<Os> =
    GenericContext<PassConnector<Os>, PassMuonSessionStore, NoInfo, PassCookieHandler<Os>>;
#[cfg(wasm_runtime)]
pub type Context<Sleeper> = GenericContext<
    PassConnector<Sleeper>,
    PassMuonSessionStore,
    NoInfo,
    PassCookieHandler<Sleeper>,
>;

#[cfg(tokio_runtime)]
pub type PassAuthClient<Os> = Client<Context<Os>>;
#[cfg(wasm_runtime)]
pub type PassAuthClient<Sleeper> = Client<Context<Sleeper>>;

#[cfg(tokio_runtime)]
pub type PassAuthSession<Os> = Session<Context<Os>>;
#[cfg(wasm_runtime)]
pub type PassAuthSession<Sleeper> = Session<Context<Sleeper>>;
