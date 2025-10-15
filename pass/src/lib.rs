#[macro_use]
extern crate tracing;

#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
mod test_tools;

mod account;
mod cache;
mod client;
mod common;
mod constants;
mod crypto;
mod feature_flags;
mod info;
mod invite;
mod item;
mod local_crypto;
mod logout;
mod pagination;
pub mod password;
mod ping;
mod share;
mod user;
mod user_keys;
mod utils;
mod vault;

pub use client::PassClient;
pub use item::create::login;
pub use item::find::FindItemQuery;
pub use user::access::{PassPlan, PlanType, UserDataSettings, UserInfo};
pub use vault::{CreateVaultArgs, UpdateVaultArgs};
