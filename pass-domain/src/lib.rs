#[macro_use]
mod macros;

pub mod crypto;
mod models;
mod protos;

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

pub use models::address::*;
pub use models::invite::*;
pub use models::item::*;
pub use models::share::*;
pub use models::vault::*;
pub use protobuf;

implement_custom_methods!(protos::vault::vault_v1::Vault);
implement_custom_methods!(protos::item::item_v1::Item);
implement_custom_methods!(protos::file::file_v1::FileMetadata);
