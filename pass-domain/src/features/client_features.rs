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

use crate::{AccountCrypto, DataStorage, FsStorage, LocalKeyProvider, PgpCrypto, TelemetryHandler};
use anyhow::Result;
use std::any::Any;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait ClientFeatures: Send + Sync + Any {
    async fn get_local_key_provider(&self) -> Result<Arc<dyn LocalKeyProvider>>;
    async fn get_account_crypto(&self) -> Arc<dyn AccountCrypto>;
    async fn get_fs(&self) -> Arc<dyn FsStorage>;
    async fn get_pgp_crypto(&self) -> Arc<dyn PgpCrypto>;
    async fn get_telemetry_handler(&self) -> Arc<dyn TelemetryHandler>;
    async fn get_data_storage(&self) -> Result<Arc<dyn DataStorage>>;
    async fn on_session_invalidated(&self) -> Result<()>;
}
