pub mod credential;
pub mod login;
pub mod whoami;

use crate::error::StoreError;
use reqwest::blocking::Client;
use serde_json::Value;

/// Our own name: the store doesn't need us to pretend to be snapcraft.
pub const USER_AGENT: &str = concat!("idiot-store/", env!("CARGO_PKG_VERSION"));

pub(crate) fn client() -> Result<Client, StoreError> {
    Ok(Client::builder().user_agent(USER_AGENT).build()?)
}

/// Extracts a human-readable message from a store or Ubuntu SSO error body:
/// a top-level `message`, else the first entry of `error_list`.
pub(crate) fn error_message(json: &Value) -> String {
    if let Some(msg) = json.get("message").and_then(Value::as_str) {
        return msg.to_string();
    }
    json.get("error_list")
        .and_then(Value::as_array)
        .and_then(|e| e.first())
        .and_then(|first| first.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("unknown error")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_top_level() {
        assert_eq!(error_message(&json!({"message": "nope"})), "nope");
    }

    #[test]
    fn message_from_error_list() {
        let body =
            json!({"error_list": [{"code": "x", "message": "first"}, {"message": "second"}]});
        assert_eq!(error_message(&body), "first");
    }

    #[test]
    fn message_unknown() {
        assert_eq!(error_message(&json!({})), "unknown error");
    }
}
