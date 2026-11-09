pub mod login;
pub mod whoami;

use idiot_store::StoreError;
use std::fmt;

/// An error with the exit status it should produce.
pub struct Failure {
    pub code: u8,
    message: String,
}

impl Failure {
    /// A usage or local error (exit 1).
    pub fn local(message: impl fmt::Display) -> Self {
        Failure {
            code: 1,
            message: message.to_string(),
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<StoreError> for Failure {
    fn from(e: StoreError) -> Self {
        Failure {
            code: e.exit_code() as u8,
            message: e.to_string(),
        }
    }
}

impl From<std::io::Error> for Failure {
    fn from(e: std::io::Error) -> Self {
        Failure::local(e)
    }
}
