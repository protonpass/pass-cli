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

use super::runtime::{AuthRuntime, ClientWithStore};
use crate::auth::config::ClientConfig;
use crate::auth::storage::SessionStorage;
use crate::domain::LocalKeyProvider;
use futures::task::{FutureObj, Spawn, SpawnError};
use muon::common::sdk::Sdk;
use muon::rt::{
    InstantFactory, Monotonic, MuonInstant, MuonSystemTime, SinceUnixEpoch as _, Sleep,
    SystemTimeFactory,
};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

/// Monotonic clock supplied by the host (e.g. JS `performance.now()`), which
/// muon uses for timeouts and retry backoff.
pub trait MonotonicClock: Send + Sync {
    /// Milliseconds since an arbitrary origin. Must never go backwards.
    fn now_ms(&self) -> f64;
}

#[derive(Clone)]
pub struct WasmTime {
    clock: Arc<dyn MonotonicClock>,
}

impl WasmTime {
    pub fn new(clock: Arc<dyn MonotonicClock>) -> Self {
        Self { clock }
    }
}

impl Sleep for WasmTime {
    type Sleep<'a>
        = Pin<Box<dyn Future<Output = ()> + 'a>>
    where
        Self: 'a;

    fn sleep(&self, duration: Duration) -> Self::Sleep<'static> {
        Box::pin(gloo_timers::future::sleep(duration))
    }
}

impl InstantFactory for WasmTime {
    type Instant = MuonInstant;

    fn now(&self) -> Self::Instant {
        let ms = self.clock.now_ms();
        MuonInstant::from_duration(Duration::from_secs_f64(ms / 1000.0))
    }
}

// SAFETY: `MonotonicClock` implementations must never go backwards.
unsafe impl Monotonic for WasmTime {}

impl SystemTimeFactory for WasmTime {
    type SystemTime = MuonSystemTime;

    fn now(&self) -> Self::SystemTime {
        MuonSystemTime::since_unix_epoch(Duration::from_secs_f64(js_sys::Date::now() / 1000.0))
    }
}

#[derive(Debug, Clone, Default)]
pub struct WasmExecutor;

impl Spawn for WasmExecutor {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        wasm_bindgen_futures::spawn_local(future);
        Ok(())
    }
}

pub type WasmContext = super::os::Context<WasmTime>;
pub type WasmClient = super::os::PassAuthClient<WasmTime>;
pub type WasmSession = super::os::PassAuthSession<WasmTime>;

/// [`AuthRuntime`] for wasm builds: muon's reqwest (fetch) transport, with
/// JS timers and `spawn_local`.
#[derive(Clone)]
pub struct WasmRuntime {
    time: WasmTime,
    executor: Arc<dyn Spawn + Send + Sync>,
}

impl WasmRuntime {
    pub fn new(clock: Arc<dyn MonotonicClock>) -> Self {
        Self {
            time: WasmTime::new(clock),
            executor: Arc::new(WasmExecutor),
        }
    }
}

impl AuthRuntime for WasmRuntime {
    type Context = WasmContext;
    type Time = WasmTime;

    fn time(&self) -> &Self::Time {
        &self.time
    }

    async fn create_client(
        &self,
        key_provider: Arc<dyn LocalKeyProvider>,
        storage: Arc<dyn SessionStorage>,
        config: &ClientConfig,
        sdk: &Sdk,
    ) -> anyhow::Result<ClientWithStore<Self::Context>> {
        super::client_builder::create_client(
            key_provider,
            storage,
            config,
            sdk.clone(),
            self.time.clone(),
            self.executor.clone(),
        )
        .await
    }
}
