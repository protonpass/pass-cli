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

use crate::auth::config::ClientConfig;
use crate::auth::cookie_store::PassCookieStore;
use crate::auth::os::PassAuthClient;
use crate::auth::storage::SessionStorage;
use crate::auth::store::{
    CustomEnv, GetStoreError, PassSessionStore, SerializedEnv, SharedPassSessionStore,
};
use crate::domain::LocalKeyProvider;
use crate::domain::headers::{ClientHeaders, HeaderBuilder};
use anyhow::{Context as _, anyhow};
use futures::task::Spawn;
use muon::app::App;
use muon::env::Environment;
use parking_lot::RwLock;
use rand::rng;
use rand_chacha::ChaCha20Rng;
use rand_chacha::rand_core::SeedableRng;
use std::sync::Arc;

#[cfg(tokio_runtime)]
use crate::auth::config::proxy_env_var;
#[cfg(tokio_runtime)]
use futures::task::SpawnExt as _;
#[cfg(tokio_runtime)]
use muon::client::builder::Hyper;
#[cfg(tokio_runtime)]
use muon::rt::{OperatingSystem, Sleep, Spawner};

#[cfg(wasm_runtime)]
use muon::rt::TimeCapabilities;
#[cfg(wasm_runtime)]
use muon::transport::http::reqwest::builder::Reqwest;

const XDEBUG_SESSION_HEADER: &str = "XDEBUG_SESSION";

fn generate_headers(config: &ClientConfig) -> ClientHeaders {
    let mut builder = HeaderBuilder::new(&config.product_name, &config.product_version);

    if let Some(ref locale) = config.locale {
        builder = builder.with_locale(locale);
    }

    for header in &config.extra_headers {
        builder = builder.with_extra_header(&header.name, &header.value);
    }

    builder.build()
}

fn get_env(config: &ClientConfig) -> SerializedEnv {
    let env_string = config
        .environment
        .clone()
        .unwrap_or_else(|| "prod".to_string());

    let env_str = env_string.as_str();

    match env_str {
        "prod" => SerializedEnv::Prod,
        "atlas" => SerializedEnv::Atlas(None),
        "localhost" => SerializedEnv::Custom(CustomEnv::Localhost),
        s if s.starts_with("http") => SerializedEnv::Custom(CustomEnv::CustomUrl(s.to_string())),
        s => SerializedEnv::Atlas(Some(s.to_string())),
    }
}

fn store_using_current_env(store_env: &Environment, current_env: &Environment) -> bool {
    match (store_env, current_env) {
        (Environment::Prod(_), Environment::Prod(_)) => true,
        (Environment::Custom(_), Environment::Custom(_)) => true,
        (Environment::Atlas(_), Environment::Atlas(_)) => true,
        (Environment::Scientist(s1), Environment::Scientist(s2)) => {
            // Compare by serializing through SerializedEnv
            let s1_serialized = SerializedEnv::from(Environment::Scientist(s1.clone()));
            let s2_serialized = SerializedEnv::from(Environment::Scientist(s2.clone()));
            matches!(
                (s1_serialized, s2_serialized),
                (SerializedEnv::Atlas(Some(a)), SerializedEnv::Atlas(Some(b))) if a == b
            )
        }
        _ => false,
    }
}

async fn load_or_create_store(
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    config: &ClientConfig,
    store_executor: Arc<dyn Spawn + Send + Sync>,
) -> anyhow::Result<(PassSessionStore, Environment)> {
    key_provider
        .get_key()
        .await
        .context("Error accessing key provider")?;

    let store = match PassSessionStore::get_from_local(
        storage.clone(),
        key_provider.clone(),
        store_executor.clone(),
    )
    .await
    {
        Ok(store) => store,
        Err(e) => {
            return match e {
                GetStoreError::CannotDecrypt(e) => Err(anyhow!(
                    "Error decrypting local session({e:#}). Make sure you have not changed your key provider / removed your local key, or try to logout and log in again"
                )),
                GetStoreError::Other(e) => Err(anyhow!("Error loading local session: {e:#}")),
            };
        }
    };

    let current_env = Environment::from(get_env(config));

    let store = store.unwrap_or_else(|| {
        PassSessionStore::new(current_env.clone(), storage, key_provider, store_executor)
    });

    if !store_using_current_env(&store.env, &current_env) {
        return Err(anyhow!(
            "ENVIRONMENT has switched! Please log out and log back in again with the new environment"
        ));
    }

    Ok((store, current_env))
}

#[cfg(tokio_runtime)]
pub async fn create_client<Os, Ex>(
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    config: &ClientConfig,
    os: Os,
    executor: Ex,
) -> anyhow::Result<(PassAuthClient<Os>, Arc<RwLock<PassSessionStore>>)>
where
    Os: OperatingSystem,
    Ex: Spawner,
    for<'a> <Os::Time as Sleep>::Sleep<'a>: Send + Sync,
{
    let app = App::new(&config.app_header)
        .with_context(|| format!("invalid app header `{}`", config.app_header))?;

    let store_executor: Arc<dyn Spawn + Send + Sync> = Arc::new(executor.clone());
    let (store, current_env) =
        load_or_create_store(key_provider, storage, config, store_executor).await?;

    let shared_store = SharedPassSessionStore::new(store);
    let store_ref = shared_store.inner.clone();
    let cookie_store = PassCookieStore::new(store_ref.clone());

    let headers = generate_headers(config);
    trace!("Generated headers: {headers:?}");

    let mut transport_builder = muon::Client::builder_with_transport::<Hyper>(app, current_env)
        .with_operating_system(os, ChaCha20Rng::from_rng(&mut rng()));

    if config.proxy_config.http_proxy.is_some() {
        info!("Using HTTP_PROXY config");
        transport_builder = transport_builder.proxy(muon::common::Proxy::Env(
            muon::common::EnvProxy::all(proxy_env_var("HTTP_PROXY")),
        ));
    }

    if config.proxy_config.https_proxy.is_some() {
        info!("Using HTTPS_PROXY config");
        transport_builder = transport_builder.proxy(muon::common::Proxy::Env(
            muon::common::EnvProxy::all(proxy_env_var("HTTPS_PROXY")),
        ));
    }

    let mut builder = transport_builder
        .with_persistence(shared_store)
        .with_cookie_store(cookie_store);

    for (name, value) in headers.to_http_headers() {
        builder = builder.with_default_headers((name, value));
    }

    if let Some(ref debug_config) = config.debug_config
        && let Some(ref session) = debug_config.xdebug_session
    {
        info!("Adding XDEBUG_SESSION header");
        builder = builder.with_default_headers((XDEBUG_SESSION_HEADER, session.clone()));
    }

    let (client, driver) = builder
        .build_with_driver()
        .context("failed to build client")?;

    let driver_executor = executor.clone();
    let driver = driver.spawn_parallel(move |task| {
        if let Err(e) = driver_executor.spawn_obj(task) {
            error!("Failed to spawn a muon background task: {e:?}");
        }
    });
    executor
        .spawn(driver)
        .context("failed to spawn muon driver")?;

    Ok((client, store_ref))
}

#[cfg(wasm_runtime)]
pub async fn create_client<Sleeper>(
    key_provider: Arc<dyn LocalKeyProvider>,
    storage: Arc<dyn SessionStorage>,
    config: &ClientConfig,
    sleeper: Sleeper,
    executor: Arc<dyn Spawn + Send + Sync>,
) -> anyhow::Result<(PassAuthClient<Sleeper>, Arc<RwLock<PassSessionStore>>)>
where
    Sleeper: TimeCapabilities,
{
    let app = App::new(&config.app_header)
        .with_context(|| format!("invalid app header `{}`", config.app_header))?;

    let (store, current_env) =
        load_or_create_store(key_provider, storage, config, executor.clone()).await?;

    let shared_store = SharedPassSessionStore::new(store);
    let store_ref = shared_store.inner.clone();
    let cookie_store = PassCookieStore::new(store_ref.clone());

    let headers = generate_headers(config);
    trace!("Generated headers: {headers:?}");

    if config.proxy_config.http_proxy.is_some() || config.proxy_config.https_proxy.is_some() {
        warn!("HTTP(S)_PROXY config is ignored on the wasm transport");
    }

    let mut builder = muon::Client::builder_with_transport::<Reqwest>(app, current_env)
        .with_operating_system(sleeper, ChaCha20Rng::from_rng(&mut rng()))
        .with_persistence(shared_store)
        .with_cookie_store(cookie_store);

    for (name, value) in headers.to_http_headers() {
        builder = builder.with_default_headers((name, value));
    }

    if let Some(ref debug_config) = config.debug_config
        && let Some(ref session) = debug_config.xdebug_session
    {
        info!("Adding XDEBUG_SESSION header");
        builder = builder.with_default_headers((XDEBUG_SESSION_HEADER, session.clone()));
    }

    let client = builder.build().context("failed to build client")?;
    Ok((client, store_ref))
}
