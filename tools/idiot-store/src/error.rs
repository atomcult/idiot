use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum StoreError {
    // Credential / format
    InvalidCredential,
    InvalidResponse,

    // Login flow
    MissingRootMacaroon,
    MissingDischargeMacaroon,
    NoCaveatForLoginUbuntu,

    // Authentication
    TwoFactorRequired,
    AuthenticationFailed(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StoreError::InvalidCredential => write!(f, "invalid credential format"),
            StoreError::InvalidResponse => write!(f, "unexpected response from store endpoint"),
            StoreError::MissingRootMacaroon => {
                write!(f, "login failed: macaroon not found in response")
            }
            StoreError::MissingDischargeMacaroon => {
                write!(f, "login failed: discharge macaroon not found in response")
            }
            StoreError::NoCaveatForLoginUbuntu => {
                write!(f, "login failed: no third-party caveat found for login.ubuntu.com")
            }
            StoreError::TwoFactorRequired => write!(f, "two-factor authentication required"),
            StoreError::AuthenticationFailed(msg) => write!(f, "authentication failed: {msg}"),
        }
    }
}

impl Error for StoreError {}
