#[macro_use]
extern crate tracing;

#[macro_use]
mod macros;

mod account;
mod cache;
mod client;
mod client_features;
mod common;
mod constants;
mod crypto;
mod info;
mod invite;
mod item;
mod local_crypto;
mod pagination;
pub mod password;
mod ping;
mod share;
mod user;
mod user_keys;
mod utils;
mod vault;

pub use account::{Passphrase, UnlockedAddressKey, UnlockedAddressKeys};
pub use client::PassClient;
pub use client_features::ClientFeatures;
pub use crypto::{PgpCrypto, PgpCryptoError, PrivateKey, PublicKey};
pub use item::create::login;
pub use item::find::FindItemQuery;
pub use muon::rest::core::v4::keys::Key as ApiKey;
pub use muon::rest::core::v4::keys::salts::KeySalt as ApiKeySalt;
pub use user_keys::UserKey;
pub use vault::{CreateVaultArgs, UpdateVaultArgs};
