//! Account details behind a credential.

use super::{client, credential, error_message};
use crate::error::StoreError;
use reqwest::StatusCode;
use serde_json::Value;

const WHOAMI_ENDPOINT: &str = "https://dashboard.snapcraft.io/api/v2/tokens/whoami";

#[derive(Debug, PartialEq)]
pub struct WhoAmI {
    pub email: String,
    pub username: String,
    pub id: String,
    pub permissions: Vec<String>,
    pub channels: Vec<String>,
    /// As the store reports it, with the `Z` it leaves off (as snapcraft).
    pub expires: Option<String>,
}

impl WhoAmI {
    /// The report, line for line as `snapcraft whoami` prints it.
    pub fn report(&self) -> String {
        let list = |v: &[String]| {
            if v.is_empty() {
                "no restrictions".to_string()
            } else {
                v.join(", ")
            }
        };
        format!(
            "email: {}\nusername: {}\nid: {}\npermissions: {}\nchannels: {}\nexpires: {}\n",
            self.email,
            self.username,
            self.id,
            list(&self.permissions),
            list(&self.channels),
            self.expires.as_deref().unwrap_or("N/A"),
        )
    }
}

/// Asks the store who `credential` belongs to.
pub fn whoami(credential: &str) -> Result<WhoAmI, StoreError> {
    let header = credential::auth_header(credential)?;
    let resp = client()?
        .get(WHOAMI_ENDPOINT)
        .header("Authorization", header)
        .header("Accept", "application/json")
        .send()?;
    match resp.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(StoreError::Rejected),
        s if s.is_success() => parse(&resp.json()?),
        _ => {
            let json: Value = resp.json().unwrap_or(Value::Null);
            Err(StoreError::Store(error_message(&json)))
        }
    }
}

fn parse(json: &Value) -> Result<WhoAmI, StoreError> {
    let bad = || StoreError::Store("unexpected whoami response".into());
    let account = json.get("account").ok_or_else(bad)?;
    let field = |k: &str| {
        account
            .get(k)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(bad)
    };
    let strings = |k: &str| -> Vec<String> {
        json.get(k)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    Ok(WhoAmI {
        email: field("email")?,
        username: field("username")?,
        id: field("id")?,
        permissions: strings("permissions"),
        channels: strings("channels"),
        expires: json
            .get("expires")
            .and_then(Value::as_str)
            .map(|s| format!("{s}Z")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn report_matches_snapcraft() {
        let body = json!({
            "account": {"email": "you@example.com", "id": "abc123", "name": "You", "username": "you"},
            "channels": null,
            "expires": "2027-11-09T18:00:00.000",
            "packages": null,
            "permissions": ["package_access", "package_push"],
        });
        assert_eq!(
            parse(&body).unwrap().report(),
            "email: you@example.com\nusername: you\nid: abc123\n\
             permissions: package_access, package_push\nchannels: no restrictions\n\
             expires: 2027-11-09T18:00:00.000Z\n"
        );
    }

    #[test]
    fn missing_account() {
        assert!(matches!(parse(&json!({})), Err(StoreError::Store(_))));
    }
}
