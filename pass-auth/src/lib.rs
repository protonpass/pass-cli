#[macro_use]
extern crate tracing;

pub mod authenticator;
pub mod callbacks;
pub mod client_builder;
pub mod config;
pub mod error;
pub mod extra_password;
pub mod interactive_login;
pub mod post_login;
pub mod service_account;
pub mod storage;
pub mod store;
pub mod web_login;

pub use authenticator::Authenticator;
pub use callbacks::{AuthEventHandler, CredentialProvider};
pub use client_builder::ENVIRONMENT_ENV_VAR;
pub use config::{ClientConfig, DebugConfig, PostLoginConfig, ProxyConfig};
pub use error::AuthError;
pub use storage::SessionStorage;
pub use store::{PassSessionStore, SharedPassSessionStore};
