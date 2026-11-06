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

use crate::PassClientContext;
use crate::auth::config::ClientConfig;
use crate::auth::storage::SessionStorage;
use crate::auth::store::PassSessionStore;
use crate::domain::LocalKeyProvider;
use anyhow::Result;
use muon::common::sdk::Sdk;
use muon::rt::Sleep;
use parking_lot::RwLock;
use std::sync::Arc;

pub type ClientWithStore<C> = (muon::Client<C>, Arc<RwLock<PassSessionStore>>);

pub trait AuthRuntime: Clone {
    /// The muon context of the clients this runtime builds.
    type Context: PassClientContext;
    type Time: Sleep;

    fn time(&self) -> &Self::Time;

    fn create_client(
        &self,
        key_provider: Arc<dyn LocalKeyProvider>,
        storage: Arc<dyn SessionStorage>,
        config: &ClientConfig,
        sdk: &Sdk,
    ) -> impl Future<Output = Result<ClientWithStore<Self::Context>>>;
}
