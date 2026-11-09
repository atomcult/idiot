//! The store's macaroon login, as craft-store does it: ask the dashboard
//! for a root macaroon with the wanted restrictions, then have
//! login.ubuntu.com discharge its third-party caveat with the user's
//! email, password and (if needed) one-time password.

use super::{client, credential, error_message};
use crate::error::StoreError;
use base64::prelude::*;
use macaroon::{Caveat, Macaroon};
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::Value;

const ACL_ENDPOINT: &str = "https://dashboard.snapcraft.io/dev/api/acl/";
const DISCHARGE_ENDPOINT: &str = "https://login.ubuntu.com/api/v2/tokens/discharge";
const SSO_LOCATION: &str = "login.ubuntu.com";

/// What the credential may do.
#[derive(Debug, Serialize)]
pub struct Restrictions {
    pub permissions: Vec<String>,
    /// Shown on the store dashboard next to the credential.
    pub description: String,
    /// RFC 3339.
    pub expires: String,
    #[serde(rename = "packages", serialize_with = "snaps")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub snaps: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub channels: Vec<String>,
}

fn snaps<S: serde::Serializer>(names: &[String], s: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Package<'a> {
        r#type: &'a str,
        name: &'a str,
    }
    s.collect_seq(names.iter().map(|name| Package {
        r#type: "snap",
        name,
    }))
}

#[derive(Serialize)]
struct DischargeRequest<'a> {
    email: &'a str,
    password: &'a str,
    caveat_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    otp: Option<&'a str>,
}

/// A login in progress: the root macaroon, waiting for its discharge.
pub struct Login {
    client: Client,
    root: Macaroon,
    caveat_id: String,
}

impl Login {
    /// Requests the root macaroon for `restrictions`.
    pub fn start(restrictions: &Restrictions) -> Result<Self, StoreError> {
        let client = client()?;
        let resp = client
            .post(ACL_ENDPOINT)
            .header("Accept", "application/json")
            .json(restrictions)
            .send()?;
        let status = resp.status();
        let json: Value = resp.json()?;
        if !status.is_success() {
            return Err(StoreError::Store(error_message(&json)));
        }
        let root = json
            .get("macaroon")
            .and_then(Value::as_str)
            .and_then(|m| Macaroon::deserialize(m).ok())
            .ok_or_else(|| StoreError::Store("no macaroon in the store's response".into()))?;
        let caveat_id = sso_caveat_id(&root)?;
        Ok(Login {
            client,
            root,
            caveat_id,
        })
    }

    /// Discharges the root macaroon and returns the credential. Fails with
    /// `TwoFactorRequired` when the account needs `otp` and none was given.
    pub fn finish(
        &self,
        email: &str,
        password: &str,
        otp: Option<&str>,
    ) -> Result<String, StoreError> {
        let resp = self
            .client
            .post(DISCHARGE_ENDPOINT)
            .header("Accept", "application/json")
            .json(&DischargeRequest {
                email,
                password,
                caveat_id: &self.caveat_id,
                otp,
            })
            .send()?;
        let status = resp.status();
        let json: Value = resp.json()?;
        if !status.is_success() {
            return Err(sso_error(&json));
        }
        let discharge = json
            .get("discharge_macaroon")
            .and_then(Value::as_str)
            .and_then(|m| Macaroon::deserialize(m).ok())
            .ok_or_else(|| StoreError::Store("no discharge macaroon in SSO's response".into()))?;
        credential::encode(&self.root, &discharge)
    }
}

/// Maps an Ubuntu SSO error body to an error. SSO reports a missing OTP as
/// `TWOFACTOR_REQUIRED`, at the top level or inside `error_list`.
fn sso_error(json: &Value) -> StoreError {
    let code = |v: &Value| v.get("code").and_then(Value::as_str) == Some("TWOFACTOR_REQUIRED");
    let nested = json
        .get("error_list")
        .and_then(Value::as_array)
        .is_some_and(|errors| errors.iter().any(code));
    if code(json) || nested {
        StoreError::TwoFactorRequired
    } else {
        StoreError::AuthenticationFailed(error_message(json))
    }
}

/// The id of the root macaroon's login.ubuntu.com caveat, which SSO wants
/// back as text.
fn sso_caveat_id(root: &Macaroon) -> Result<String, StoreError> {
    root.third_party_caveats()
        .into_iter()
        .find_map(|c| match c {
            Caveat::ThirdParty(tp) if tp.location() == SSO_LOCATION => Some(tp),
            _ => None,
        })
        .and_then(|tp| BASE64_STANDARD.decode(tp.id().to_string()).ok())
        .and_then(|id| String::from_utf8(id).ok())
        .ok_or_else(|| StoreError::Store("no login.ubuntu.com caveat in the root macaroon".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::credential::tests::macaroon;
    use macaroon::MacaroonKey;
    use serde_json::json;

    #[test]
    fn caveat_id() {
        let mut root = macaroon("dashboard.snapcraft.io", "root-id");
        let key: MacaroonKey = (&[9u8; 32]).into();
        root.add_first_party_caveat("expires < 2099".into());
        root.add_third_party_caveat("elsewhere.example", &key, "not-this".into());
        root.add_third_party_caveat(SSO_LOCATION, &key, "{\"sso\":\"caveat\"}".into());
        assert_eq!(sso_caveat_id(&root).unwrap(), "{\"sso\":\"caveat\"}");
    }

    #[test]
    fn caveat_missing() {
        let root = macaroon("dashboard.snapcraft.io", "root-id");
        assert!(matches!(sso_caveat_id(&root), Err(StoreError::Store(_))));
    }

    // recorded Ubuntu SSO response bodies
    #[test]
    fn two_factor_top_level() {
        let body = json!({"code": "TWOFACTOR_REQUIRED", "message": "2-factor authentication required.", "extra": {}});
        assert!(matches!(sso_error(&body), StoreError::TwoFactorRequired));
    }

    #[test]
    fn two_factor_nested() {
        let body = json!({"error_list": [{"code": "TWOFACTOR_REQUIRED", "message": "2-factor authentication required."}]});
        assert!(matches!(sso_error(&body), StoreError::TwoFactorRequired));
    }

    #[test]
    fn bad_password() {
        let body = json!({"code": "INVALID_CREDENTIALS", "message": "Provided email/password is not correct.", "extra": {}});
        match sso_error(&body) {
            StoreError::AuthenticationFailed(msg) => {
                assert_eq!(msg, "Provided email/password is not correct.")
            }
            e => panic!("unexpected {e:?}"),
        }
    }

    #[test]
    fn restrictions_json() {
        let r = Restrictions {
            permissions: vec!["package_push".into()],
            description: "idiot@host".into(),
            expires: "2027-01-01T00:00:00+00:00".into(),
            snaps: vec!["my-gadget".into()],
            channels: vec![],
        };
        assert_eq!(
            serde_json::to_value(&r).unwrap(),
            json!({
                "permissions": ["package_push"],
                "description": "idiot@host",
                "expires": "2027-01-01T00:00:00+00:00",
                "packages": [{"type": "snap", "name": "my-gadget"}],
            })
        );
    }
}
