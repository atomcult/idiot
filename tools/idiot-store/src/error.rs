use std::fmt;

/// Everything the store client can fail with. `exit_code` maps each to the
/// helper's documented exit status: 2 authentication, 3 network or store.
#[derive(Debug)]
pub enum StoreError {
    /// The credential isn't the `u1-macaroon` format snapcraft writes.
    InvalidCredential,
    /// Ubuntu SSO wants a one-time password (the caller prompts and retries).
    TwoFactorRequired,
    /// Ubuntu SSO refused the email, password or one-time password.
    AuthenticationFailed(String),
    /// The store refused the credential (expired, revoked, malformed).
    Rejected,
    /// The store answered with an error, or something we can't read.
    Store(String),
    /// The request never got an answer.
    Network(String),
}

impl StoreError {
    pub fn exit_code(&self) -> i32 {
        match self {
            StoreError::InvalidCredential
            | StoreError::TwoFactorRequired
            | StoreError::AuthenticationFailed(_)
            | StoreError::Rejected => 2,
            StoreError::Store(_) | StoreError::Network(_) => 3,
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StoreError::InvalidCredential => write!(f, "invalid credential format"),
            StoreError::TwoFactorRequired => write!(f, "two-factor authentication required"),
            StoreError::AuthenticationFailed(msg) => write!(f, "authentication failed: {msg}"),
            StoreError::Rejected => {
                write!(f, "the store rejected the credential (expired or revoked?)")
            }
            StoreError::Store(msg) => write!(f, "store error: {msg}"),
            StoreError::Network(msg) => write!(f, "network error: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<reqwest::Error> for StoreError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            StoreError::Store(format!("unreadable response: {e}"))
        } else {
            StoreError::Network(e.to_string())
        }
    }
}
