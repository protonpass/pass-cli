use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("CTAP error: {0}")]
    CtapError(String),

    #[error("No authenticator found")]
    NoAuthenticator,

    #[error("Authentication cancelled or failed")]
    AuthenticationFailed,

    #[error("PIN required but not provided")]
    PinRequired,

    #[error("Invalid PIN")]
    InvalidPin,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for AuthError {
    fn from(err: anyhow::Error) -> Self {
        AuthError::CtapError(err.to_string())
    }
}
