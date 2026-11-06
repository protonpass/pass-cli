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
use async_compat::Compat;
use futures::TryFutureExt;
use muon::common::sdk::Sdk;
use muon::rt::{
    InstantFactory, Monotonic, MuonInstant, MuonSystemTime, OperatingSystem, Resolve,
    SinceUnixEpoch as _, Sleep, Spawner, SystemTimeFactory, TcpConnect,
};
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TokioTime {
    at_start: std::time::Instant,
}

impl Default for TokioTime {
    fn default() -> Self {
        Self {
            at_start: std::time::Instant::now(),
        }
    }
}

impl Sleep for TokioTime {
    type Sleep<'a>
        = Pin<Box<dyn Future<Output = ()> + Send + Sync + 'a>>
    where
        Self: 'a;

    fn sleep(&self, duration: core::time::Duration) -> Self::Sleep<'static> {
        Box::pin(tokio::time::sleep(duration))
    }
}

impl InstantFactory for TokioTime {
    type Instant = MuonInstant;

    fn now(&self) -> Self::Instant {
        MuonInstant::from_duration(std::time::Instant::now() - self.at_start)
    }
}

unsafe impl Monotonic for TokioTime {}

impl SystemTimeFactory for TokioTime {
    type SystemTime = MuonSystemTime;

    fn now(&self) -> Self::SystemTime {
        MuonSystemTime::since_unix_epoch(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("failed to get time"),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokioTcpConnector;

impl TcpConnect for TokioTcpConnector {
    type Err = std::io::Error;
    type Socket = Compat<tokio::net::TcpStream>;

    async fn tcp_connect(&self, addr: core::net::SocketAddr) -> Result<Self::Socket, Self::Err> {
        tokio::net::TcpStream::connect(addr).await.map(Compat::new)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokioResolver;

impl Resolve for TokioResolver {
    type Err = std::io::Error;

    fn resolve(
        &self,
        host: &str,
    ) -> impl Future<Output = Result<Vec<core::net::IpAddr>, Self::Err>> {
        tokio::net::lookup_host(format!("{host}:80"))
            .map_ok(|addresses| addresses.map(|addr| addr.ip()).collect())
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokioOs {
    time: TokioTime,
    tcp: TokioTcpConnector,
    resolver: TokioResolver,
}

impl OperatingSystem for TokioOs {
    type Time = TokioTime;
    fn get_time_capabilities(&self) -> &Self::Time {
        &self.time
    }

    type TcpConnector = TokioTcpConnector;
    fn get_tcp_connector(&self) -> &Self::TcpConnector {
        &self.tcp
    }

    type Resolver = TokioResolver;
    fn get_resolver(&self) -> &Self::Resolver {
        &self.resolver
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokioExecutor;

impl futures::task::Spawn for TokioExecutor {
    fn spawn_obj(
        &self,
        future: futures::task::FutureObj<'static, ()>,
    ) -> Result<(), futures::task::SpawnError> {
        let fut = tokio::spawn(future);
        drop(fut);
        Ok(())
    }
}

pub type TokioContext = super::os::Context<TokioOs>;
pub type TokioClient = super::os::PassAuthClient<TokioOs>;
pub type TokioSession = super::os::PassAuthSession<TokioOs>;

/// [`AuthRuntime`] for native builds: muon's hyper transport on top of an
/// [`OperatingSystem`] and a multi-thread executor. The defaults are the
/// tokio-backed implementations.
#[derive(Debug, Clone, Default)]
pub struct TokioRuntime<Os = TokioOs, Ex = TokioExecutor> {
    pub os: Os,
    pub executor: Ex,
}

impl<Os, Ex> AuthRuntime for TokioRuntime<Os, Ex>
where
    Os: OperatingSystem,
    Ex: Spawner,
    for<'a> <Os::Time as Sleep>::Sleep<'a>: Send + Sync,
{
    type Context = super::os::Context<Os>;
    type Time = Os::Time;

    fn time(&self) -> &Self::Time {
        self.os.get_time_capabilities()
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
            self.os.clone(),
            self.executor.clone(),
        )
        .await
    }
}
