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
mod events;
mod feature_flags;
mod first_time_setup;
mod folder;
mod info;
mod invite;
mod item;
mod local_crypto;
mod logout;
mod pagination;
pub mod password;
mod permission;
mod ping;
mod share;
mod telemetry;
mod user;
mod user_keys;
mod utils;
mod vault;

pub use account::settings::AccountUserSettings;
pub use client::{Client, PassClient, PassSessionKeyType};
pub use first_time_setup::FirstTimeSetupKey;
pub use folder::create::CreateFolderPayload;
pub use item::create::credit_card;
pub use item::create::custom;
pub use item::create::identity;
pub use item::create::login;
pub use item::create::note;
pub use item::create::ssh_key;
pub use item::create::wifi;
pub use item::find::FindItemQuery;
pub use user::access::{PassPlan, PlanType, UserDataSettings, UserInfo};
pub use utils::is_id;
pub use vault::{CreateVaultArgs, UpdateVaultArgs};
