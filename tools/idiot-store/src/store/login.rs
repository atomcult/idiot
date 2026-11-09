use crate::error::StoreError;
use base64::prelude::*;
use macaroon::{Caveat, Macaroon};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use std::error::Error;

const SNAPCRAFT_ACL_ENDPOINT: &str = "https://dashboard.snapcraft.io/dev/api/acl/";
const LOGIN_UBUNTU_ENDPOINT: &str = "https://login.ubuntu.com/api/v2/tokens/discharge";

#[derive(Debug, Serialize)]
struct Package {
    r#type: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct MacaroonCredential {
    #[serde(rename = "t")]
    token_type: String,
    #[serde(rename = "v")]
    value: MacaroonPair,
}

#[derive(Debug, Serialize)]
struct MacaroonPair {
    #[serde(rename = "r")]
    root: String,
    #[serde(rename = "d")]
    discharge: String,
}

#[derive(Serialize)]
struct MacaroonRequest {
    permissions: Vec<String>,
    description: String,
    expires: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    packages: Vec<Package>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    channels: Vec<String>,
}

#[derive(Serialize)]
struct DischargeRequest {
    email: String,
    password: String,
    caveat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    otp: Option<String>,
}

/// Parsed details from a stored credential.
pub struct CredentialInfo {
    pub token_type: String,
    pub caveats: Vec<String>,
}

/// Decodes a base64 credential string (as produced by `login`) and returns its details.
pub fn parse_credential(credential: &str) -> Result<CredentialInfo, Box<dyn Error>> {
    let decoded = BASE64_STANDARD.decode(credential.trim())?;
    let json: Value = serde_json::from_slice(&decoded)?;

    let token_type = json["t"]
        .as_str()
        .ok_or(StoreError::InvalidCredential)?
        .to_string();

    let root_str = json["v"]["r"]
        .as_str()
        .ok_or(StoreError::InvalidCredential)?;

    let root_macaroon = Macaroon::deserialize(root_str)?;

    let caveats = root_macaroon
        .first_party_caveats()
        .into_iter()
        .filter_map(|c| match c {
            Caveat::FirstParty(fp) => {
                Some(String::from_utf8_lossy(fp.predicate().as_ref()).into_owned())
            }
            _ => None,
        })
        .collect();

    Ok(CredentialInfo { token_type, caveats })
}

/// Logs in to the Snapcraft store and returns a base64 encoded macaroon credential.
///
/// Pass `otp` when two-factor authentication is required; use `None` for the first attempt.
pub async fn login(
    email: String,
    password: String,
    otp: Option<String>,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::ClientBuilder::new()
        .user_agent("snapcraft/8.6.3 ubuntu/24.04 (x86_64)")
        .build()?;

    let root_macaroon = get_root_macaroon(&client).await?;
    let discharge_macaroon =
        get_discharge_macaroon(&client, email, password, otp, &root_macaroon).await?;
    let credential = build_macaroon_credential(&root_macaroon, &discharge_macaroon)?;

    Ok(BASE64_STANDARD.encode(&credential))
}

/// Requests the root macaroon from the Snapcraft store.
async fn get_root_macaroon(client: &Client) -> Result<Macaroon, Box<dyn Error>> {
    // FIXME: dynamic expiration, packages, and channels (among other optional fields)
    let payload = MacaroonRequest {
        permissions: vec![String::from("package_access")],
        description: String::from("snapcraft@ubuntu"),
        expires: String::from("2026-02-22T17:47:58+00:00"),
        packages: Vec::new(),
        channels: Vec::new(),
    };

    let resp = client
        .post(SNAPCRAFT_ACL_ENDPOINT)
        .header("Accept", "application/json")
        .header("Connection", "keep-alive")
        .json(&payload)
        .send()
        .await?;

    let json = resp.json::<Value>().await?;

    let macaroon_str = json
        .get("macaroon")
        .ok_or(StoreError::MissingRootMacaroon)?
        .as_str()
        .ok_or(StoreError::MissingRootMacaroon)?;

    Ok(Macaroon::deserialize(macaroon_str)?)
}

/// Discharges the third-party caveat to obtain a discharge macaroon from login.ubuntu.com.
async fn get_discharge_macaroon(
    client: &Client,
    email: String,
    password: String,
    otp: Option<String>,
    root_macaroon: &Macaroon,
) -> Result<Macaroon, Box<dyn Error>> {
    let caveat_id = extract_caveat_id(root_macaroon)?;

    let payload = DischargeRequest {
        email,
        password,
        caveat_id,
        otp,
    };

    let resp = client
        .post(LOGIN_UBUNTU_ENDPOINT)
        .header("Accept", "application/json")
        .header("Connection", "keep-alive")
        .json(&payload)
        .send()
        .await?;

    let status = resp.status();
    let json = resp.json::<Value>().await?;

    if !status.is_success() {
        if two_factor_required(&json) {
            return Err(Box::new(StoreError::TwoFactorRequired));
        }
        return Err(Box::new(StoreError::AuthenticationFailed(
            extract_error_message(&json),
        )));
    }

    let discharge_str = json["discharge_macaroon"]
        .as_str()
        .ok_or(StoreError::MissingDischargeMacaroon)?;

    Ok(Macaroon::deserialize(discharge_str)?)
}

/// Returns true if the response body indicates that TOTP is required.
///
/// Ubuntu SSO may report this at the top level (`{"code": "TWOFACTOR_REQUIRED"}`) or
/// nested inside an `error_list` array.
fn two_factor_required(json: &Value) -> bool {
    if json.get("code").and_then(Value::as_str) == Some("TWOFACTOR_REQUIRED") {
        return true;
    }
    json.get("error_list")
        .and_then(Value::as_array)
        .map_or(false, |errors| {
            errors
                .iter()
                .any(|e| e.get("code").and_then(Value::as_str) == Some("TWOFACTOR_REQUIRED"))
        })
}

/// Extracts a human-readable message from an Ubuntu SSO error response.
fn extract_error_message(json: &Value) -> String {
    if let Some(msg) = json.get("message").and_then(Value::as_str) {
        return msg.to_string();
    }
    if let Some(first) = json
        .get("error_list")
        .and_then(Value::as_array)
        .and_then(|e| e.first())
    {
        if let Some(msg) = first.get("message").and_then(Value::as_str) {
            return msg.to_string();
        }
    }
    "unknown error".to_string()
}

/// Extracts and decodes the login.ubuntu.com caveat ID from the root macaroon.
fn extract_caveat_id(macaroon: &Macaroon) -> Result<String, Box<dyn Error>> {
    for caveat in macaroon.third_party_caveats() {
        let caveat = match caveat {
            Caveat::ThirdParty(c) => c,
            _ => continue,
        };

        if caveat.location() == "login.ubuntu.com" {
            let decoded = BASE64_STANDARD.decode(caveat.id().to_string())?;
            return Ok(String::from_utf8(decoded)?);
        }
    }

    Err(Box::new(StoreError::NoCaveatForLoginUbuntu))
}

/// Combines the root and discharge macaroons into a serialized JSON credential string.
fn build_macaroon_credential(
    root: &Macaroon,
    discharge: &Macaroon,
) -> Result<String, Box<dyn Error>> {
    let credential = MacaroonCredential {
        token_type: String::from("u1-macaroon"),
        value: MacaroonPair {
            root: root.serialize(macaroon::Format::V1)?,
            discharge: discharge.serialize(macaroon::Format::V1)?,
        },
    };

    Ok(serde_json::to_string(&credential)?)
}
