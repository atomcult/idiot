use crate::error::StoreError;
use base64::prelude::*;
use macaroon::Macaroon;
use reqwest::Client;
use serde_json::Value;
use std::error::Error;

const WHOAMI_ENDPOINT: &str = "https://dashboard.snapcraft.io/api/v2/tokens/whoami";

#[derive(Debug)]
pub struct Account {
    pub email: String,
    pub id: String,
    pub name: String,
    pub username: String,
}

#[derive(Debug)]
pub struct WhoAmI {
    pub account: Account,
    pub permissions: Vec<String>,
    pub channels: Vec<String>,
    /// Expiry timestamp from the server, with a trailing `Z` appended.
    pub expires: Option<String>,
}

/// Authenticates with the Snap Store using the given credential and returns account details.
///
/// `credential` is the base64 string as stored in `UBUNTU_STORE_AUTH`.
pub async fn whoami(credential: &str) -> Result<WhoAmI, Box<dyn Error>> {
    let auth_header = build_auth_header(credential)?;

    let client = Client::builder()
        .user_agent("snapcraft/8.6.3 ubuntu/24.04 (x86_64)")
        .build()?;

    let resp = client
        .get(WHOAMI_ENDPOINT)
        .header("Authorization", auth_header)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(Box::new(StoreError::AuthenticationFailed(
            "credential may be expired".to_string(),
        )));
    }

    let json = resp.json::<Value>().await?;
    parse_response(&json)
}

/// Builds the `Macaroon` Authorization header value from a stored credential.
///
/// The stored credential format is `base64({"t":"u1-macaroon","v":{"r":"...","d":"..."}})`.
/// The Snap Store expects `Macaroon <base64({"r":"...","d":"..."})>`.
fn build_auth_header(credential: &str) -> Result<String, Box<dyn Error>> {
    let decoded = BASE64_STANDARD.decode(credential.trim())?;
    let json: Value = serde_json::from_slice(&decoded)?;

    let root_str = json["v"]["r"]
        .as_str()
        .ok_or(StoreError::InvalidCredential)?;
    let discharge_str = json["v"]["d"]
        .as_str()
        .ok_or(StoreError::InvalidCredential)?;

    let root_macaroon = Macaroon::deserialize(root_str)?;
    let mut discharge_macaroon = Macaroon::deserialize(discharge_str)?;
    root_macaroon.bind(&mut discharge_macaroon);
    let bound_discharge = discharge_macaroon.serialize(macaroon::Format::V1)?;

    Ok(format!("Macaroon root={root_str}, discharge={bound_discharge}"))
}

fn parse_response(json: &Value) -> Result<WhoAmI, Box<dyn Error>> {
    let account_json = json.get("account").ok_or(StoreError::InvalidResponse)?;

    let account = Account {
        email: account_json["email"]
            .as_str()
            .ok_or(StoreError::InvalidResponse)?
            .to_string(),
        id: account_json["id"]
            .as_str()
            .ok_or(StoreError::InvalidResponse)?
            .to_string(),
        name: account_json["name"]
            .as_str()
            .ok_or(StoreError::InvalidResponse)?
            .to_string(),
        username: account_json["username"]
            .as_str()
            .ok_or(StoreError::InvalidResponse)?
            .to_string(),
    };

    let permissions = string_array(&json["permissions"]);
    let channels = string_array(&json["channels"]);
    let expires = json
        .get("expires")
        .and_then(Value::as_str)
        .map(|s| format!("{s}Z"));

    Ok(WhoAmI {
        account,
        permissions,
        channels,
        expires,
    })
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
