#[macro_use]
mod macros;

pub mod crypto;
mod feature_flag;
mod features;
mod models;
mod protos;
pub mod telemetry;
mod types;
pub mod utils;

macro_rules! implement_custom_methods {
    ($t:ty) => {
        impl $t {
            pub fn to_vec(&self) -> Result<Vec<u8>, protobuf::Error> {
                use protobuf::Message;

                let mut res = Vec::new();
                self.write_to_vec(&mut res)?;
                Ok(res)
            }

            pub fn decode_from_vec(source: Vec<u8>) -> Result<Self, protobuf::Error> {
                Self::decode_from_slice(&source)
            }

            pub fn decode_from_slice(source: &[u8]) -> Result<Self, protobuf::Error> {
                use protobuf::Message;

                Self::parse_from_bytes(source)
            }
        }

        impl Eq for $t {}
    };
}

pub use aes_gcm;
pub use feature_flag::*;
pub use features::*;
pub use models::address::*;
pub use models::events::*;
pub use models::folder::*;
pub use models::group::*;
pub use models::invite::*;
pub use models::item::*;
pub use models::share::*;
pub use models::vault::*;
pub use protobuf;
pub use telemetry::*;
pub use types::*;

implement_custom_methods!(protos::vault::vault_v1::Vault);
implement_custom_methods!(protos::item::item_v1::Item);
implement_custom_methods!(protos::file::file_v1::FileMetadata);
implement_custom_methods!(protos::folder::folder_v1::Folder);
