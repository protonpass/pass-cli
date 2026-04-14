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

use async_compat::Compat;
use futures::TryFutureExt;
use muon::common::GenericContext;
use muon::cookie_store::NoOpCookieStore;
use muon::rt::{
    InstantFactory, Monotonic, MuonInstant, MuonSystemTime, OperatingSystem, Resolve, SendExecutor,
    SinceUnixEpoch as _, Sleep, SystemTimeFactory, TcpConnect,
};
use muon::transport::http::hyper::builder::Hyper;
use muon::transport::http::hyper::connector::HyperConnector;
use muon::{Client, NoInfo, Session};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct ProdTime {
    at_start: std::time::Instant,
}

impl Default for ProdTime {
    fn default() -> Self {
        Self {
            at_start: std::time::Instant::now(),
        }
    }
}

impl Sleep for ProdTime {
    type Sleep<'a>
        = Pin<Box<dyn Future<Output = ()> + Send + Sync + 'a>>
    where
        Self: 'a;

    fn sleep(&self, duration: core::time::Duration) -> Self::Sleep<'static> {
        Box::pin(tokio::time::sleep(duration))
    }
}

impl InstantFactory for ProdTime {
    type Instant = MuonInstant;

    fn now(&self) -> Self::Instant {
        MuonInstant::from_duration(std::time::Instant::now() - self.at_start)
    }
}

unsafe impl Monotonic for ProdTime {}

impl SystemTimeFactory for ProdTime {
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
pub struct ProdOs {
    time: ProdTime,
    tcp: TokioTcpConnector,
    resolver: TokioResolver,
}

impl OperatingSystem for ProdOs {
    type Time = ProdTime;
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

// Executor using tokio::spawn
#[derive(Debug, Clone)]
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

// Type aliases for production use
pub type ProdSendExecutor = SendExecutor<TokioExecutor>;
pub type ProdConnector = HyperConnector<ProdOs, ProdSendExecutor>;
pub type ProdContext =
    GenericContext<ProdConnector, crate::store::SharedPassSessionStore, NoInfo, NoOpCookieStore>;
pub type ProdClient = Client<ProdContext>;
pub type ProdSession = Session<ProdContext>;

// Build the Hyper transport type alias
pub type ProdHyper = Hyper<ProdOs, ProdSendExecutor>;
