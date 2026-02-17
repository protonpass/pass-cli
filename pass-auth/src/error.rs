#[derive(Debug)]
pub enum AuthError {
    CannotDecrypt(anyhow::Error),
    BadExtraPassword,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for AuthError {
    fn from(e: anyhow::Error) -> Self {
        AuthError::Other(e)
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::CannotDecrypt(e) => write!(f, "Cannot decrypt session: {e:#}"),
            AuthError::BadExtraPassword => write!(f, "Incorrect extra password"),
            AuthError::Other(e) => write!(f, "{e:#}"),
        }
    }
}

impl std::error::Error for AuthError {}
