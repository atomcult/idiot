//! The credential format `snapcraft export-login` writes:
//! `base64({"t":"u1-macaroon","v":{"r":<root>,"d":<discharge>}})`, both
//! macaroons in V1 serialisation, the discharge not yet bound, and neither
//! base64-padded: snapd decodes them as unpadded URL-safe base64 and, when
//! it can't, silently sends store requests without authorisation.

use crate::error::StoreError;
use base64::prelude::*;
use macaroon::{Caveat, Format, Macaroon};
use serde::{Deserialize, Serialize};

const TOKEN_TYPE: &str = "u1-macaroon";

#[derive(Serialize, Deserialize)]
struct Credential {
    t: String,
    v: Pair,
}

#[derive(Serialize, Deserialize)]
struct Pair {
    r: String,
    d: String,
}

/// Encodes a root and discharge macaroon, serialised as the store and SSO
/// sent them, as a credential. Base64 padding is stripped, in case.
pub fn encode(root: &str, discharge: &str) -> Result<String, StoreError> {
    let json = serde_json::to_string(&Credential {
        t: TOKEN_TYPE.to_string(),
        v: Pair {
            r: unpad(root).to_string(),
            d: unpad(discharge).to_string(),
        },
    })
    .map_err(|e| StoreError::Store(e.to_string()))?;
    Ok(BASE64_STANDARD.encode(json))
}

fn unpad(serialised: &str) -> &str {
    serialised.trim_end_matches('=')
}

/// The credential without base64 padding in its macaroons, or None when it
/// has none. Lossless: padding carries no data. Fails on anything that
/// isn't a `u1-macaroon` credential.
pub fn repair(credential: &str) -> Result<Option<String>, StoreError> {
    let (root, discharge) = decode(credential)?;
    if unpad(&root) == root && unpad(&discharge) == discharge {
        return Ok(None);
    }
    encode(&root, &discharge).map(Some)
}

/// Decodes a credential into its serialised root and discharge macaroons.
pub fn decode(credential: &str) -> Result<(String, String), StoreError> {
    let bytes = BASE64_STANDARD
        .decode(credential.trim())
        .map_err(|_| StoreError::InvalidCredential)?;
    let cred: Credential =
        serde_json::from_slice(&bytes).map_err(|_| StoreError::InvalidCredential)?;
    if cred.t != TOKEN_TYPE {
        return Err(StoreError::InvalidCredential);
    }
    Ok((cred.v.r, cred.v.d))
}

/// The `Authorization` header value for a credential, as snapd builds it:
/// the root macaroon and the discharge bound to it, neither base64-padded.
pub fn auth_header(credential: &str) -> Result<String, StoreError> {
    let (root_str, discharge_str) = decode(credential)?;
    let root = Macaroon::deserialize(&root_str).map_err(|_| StoreError::InvalidCredential)?;
    let mut discharge =
        Macaroon::deserialize(&discharge_str).map_err(|_| StoreError::InvalidCredential)?;
    root.bind(&mut discharge);
    let bound = discharge
        .serialize(Format::V1)
        .map_err(|_| StoreError::InvalidCredential)?;
    Ok(format!(
        "Macaroon root={}, discharge={}",
        unpad(&root_str),
        unpad(&bound)
    ))
}

/// The permissions in a credential's root macaroon: the JSON list in its
/// `<location>|acl|[...]` first-party caveat. Offline. Empty when there is
/// no such caveat; an unreadable list is an invalid credential.
pub fn permissions(credential: &str) -> Result<Vec<String>, StoreError> {
    let (root, _) = decode(credential)?;
    let m = Macaroon::deserialize(&root).map_err(|_| StoreError::InvalidCredential)?;
    for caveat in m.first_party_caveats() {
        let Caveat::FirstParty(fp) = caveat else {
            continue;
        };
        let predicate = fp.predicate().0;
        let Ok(text) = std::str::from_utf8(&predicate) else {
            continue;
        };
        let mut parts = text.splitn(3, '|');
        if let (Some(_), Some("acl"), Some(list)) = (parts.next(), parts.next(), parts.next()) {
            return serde_json::from_str(list).map_err(|_| StoreError::InvalidCredential);
        }
    }
    Ok(Vec::new())
}

/// The credential with its discharge replaced (after a refresh).
pub fn with_discharge(credential: &str, discharge: &str) -> Result<String, StoreError> {
    let (root, _) = decode(credential)?;
    encode(&root, discharge)
}

/// The expiry times in a credential's first-party caveats
/// (`<location>|expires|<time>`), as `("root" | "discharge", time)`, without
/// contacting anything. A time without a zone gets `Z` (the store's times
/// are UTC). Nothing else from the caveats is returned: some identify the
/// account. A value that isn't a plain timestamp is skipped.
pub fn expiries(credential: &str) -> Result<Vec<(&'static str, String)>, StoreError> {
    let (root, discharge) = decode(credential)?;
    let mut out = Vec::new();
    for (label, serialised) in [("root", root), ("discharge", discharge)] {
        let m = Macaroon::deserialize(&serialised).map_err(|_| StoreError::InvalidCredential)?;
        for caveat in m.first_party_caveats() {
            let Caveat::FirstParty(fp) = caveat else {
                continue;
            };
            if let Some(time) = expiry_value(&fp.predicate().0) {
                out.push((label, time));
            }
        }
    }
    Ok(out)
}

/// The time in an `<location>|expires|<time>` predicate, zone added.
fn expiry_value(predicate: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(predicate).ok()?;
    let mut parts = text.splitn(3, '|');
    let (_location, key, time) = (parts.next()?, parts.next()?, parts.next()?);
    if key != "expires"
        || time.is_empty()
        || !time
            .chars()
            .all(|c| c.is_ascii_digit() || "-:.T+Zz".contains(c))
    {
        return None;
    }
    let zoned = time.ends_with(['Z', 'z'])
        || time
            .split_once('T')
            .is_some_and(|(_, clock)| clock.contains(['+', '-']));
    Some(if zoned {
        time.to_string()
    } else {
        format!("{time}Z")
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use macaroon::MacaroonKey;

    pub(crate) fn macaroon(location: &str, id: &str) -> Macaroon {
        macaroon::initialize().unwrap();
        let key: MacaroonKey = (&[7u8; 32]).into();
        Macaroon::create(Some(location.into()), &key, id.into()).unwrap()
    }

    pub(crate) fn ser(m: &Macaroon) -> String {
        m.serialize(Format::V1).unwrap()
    }

    /// A macaroon whose V1 serialisation the crate pads, as it did when
    /// idiot-store wrote credentials with it.
    fn padded() -> Macaroon {
        (0..)
            .map(|n| macaroon("dashboard.snapcraft.io", &format!("root-{n}")))
            .find(|m| ser(m).ends_with('='))
            .unwrap()
    }

    #[test]
    fn round_trip() {
        let root = macaroon("dashboard.snapcraft.io", "root-id");
        let discharge = macaroon("login.ubuntu.com", "discharge-id");
        let cred = encode(&ser(&root), &ser(&discharge)).unwrap();

        let (r, d) = decode(&cred).unwrap();
        assert_eq!(Macaroon::deserialize(&r).unwrap(), root);
        assert_eq!(Macaroon::deserialize(&d).unwrap(), discharge);
        // trailing newline from a file is fine
        assert!(decode(&format!("{cred}\n")).is_ok());
    }

    #[test]
    fn never_padded() {
        let root = padded();
        let discharge = macaroon("login.ubuntu.com", "discharge-id");
        let cred = encode(&ser(&root), &ser(&discharge)).unwrap();
        let (r, d) = decode(&cred).unwrap();
        assert!(!r.ends_with('=') && !d.ends_with('='));
        // still the same macaroons
        assert_eq!(Macaroon::deserialize(&r).unwrap(), root);
        assert_eq!(Macaroon::deserialize(&d).unwrap(), discharge);
    }

    #[test]
    fn kept_verbatim() {
        let (r, d) = ("MDAxY2xvY2F0aW9u", "MDAxNWxvY2F0aW9u");
        assert_eq!(
            decode(&encode(r, d).unwrap()).unwrap(),
            (r.into(), d.into())
        );
    }

    /// A credential as idiot-store wrote it before: macaroons as padded.
    fn padded_credential(root: &Macaroon) -> String {
        let json = serde_json::to_string(&Credential {
            t: TOKEN_TYPE.into(),
            v: Pair {
                r: ser(root),
                d: ser(&macaroon("login.ubuntu.com", "discharge-id")),
            },
        })
        .unwrap();
        BASE64_STANDARD.encode(json)
    }

    #[test]
    fn repair_strips_padding_only() {
        let root = padded();
        let old = padded_credential(&root);
        let fixed = repair(&old).unwrap().expect("padded credential repaired");
        let (r, d) = decode(&fixed).unwrap();
        let (old_r, old_d) = decode(&old).unwrap();
        assert_eq!(r, old_r.trim_end_matches('='));
        assert_eq!(d, old_d.trim_end_matches('='));
        assert_eq!(Macaroon::deserialize(&r).unwrap(), root);
        // a repaired credential needs no more repair
        assert!(repair(&fixed).unwrap().is_none());
    }

    #[test]
    fn repair_rejects_other_formats() {
        let other = BASE64_STANDARD.encode(r#"{"t":"other","v":{"r":"x=","d":"y"}}"#);
        assert!(matches!(repair(&other), Err(StoreError::InvalidCredential)));
        assert!(repair("not base64!").is_err());
    }

    fn with_caveat(predicate: &str) -> String {
        let mut root = macaroon("dashboard.snapcraft.io", "root-id");
        root.add_first_party_caveat(predicate.into());
        encode(&ser(&root), &ser(&macaroon("login.ubuntu.com", "d"))).unwrap()
    }

    #[test]
    fn permissions_from_acl() {
        let cred =
            with_caveat(r#"myapps.developer.ubuntu.com|acl|["package_access", "store_admin"]"#);
        assert_eq!(
            permissions(&cred).unwrap(),
            ["package_access", "store_admin"]
        );
    }

    #[test]
    fn permissions_absent_or_malformed() {
        let none = with_caveat("myapps.developer.ubuntu.com|expires|2027-01-01T00:00:00");
        assert!(permissions(&none).unwrap().is_empty());
        let bad = with_caveat("myapps.developer.ubuntu.com|acl|not json");
        assert!(matches!(
            permissions(&bad),
            Err(StoreError::InvalidCredential)
        ));
    }

    #[test]
    fn header_unpadded_and_bound() {
        let root = padded();
        let discharge = macaroon("login.ubuntu.com", "discharge-id");
        // as an older idiot-store saved it: both padded where they pad
        let cred = padded_credential(&root);
        let header = auth_header(&cred).unwrap();
        let (r, d) = header
            .strip_prefix("Macaroon root=")
            .unwrap()
            .split_once(", discharge=")
            .unwrap();
        assert!(!r.contains('=') && !d.contains('='));
        let mut bound = discharge.clone();
        root.bind(&mut bound);
        assert_eq!(d, ser(&bound).trim_end_matches('='));
    }

    #[test]
    fn snapcraft_layout() {
        let cred = encode(&ser(&macaroon("a", "r")), &ser(&macaroon("b", "d"))).unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&BASE64_STANDARD.decode(cred).unwrap()).unwrap();
        assert_eq!(json["t"], "u1-macaroon");
        assert!(json["v"]["r"].is_string() && json["v"]["d"].is_string());
    }

    #[test]
    fn rejects_garbage() {
        assert!(matches!(
            decode("not base64!"),
            Err(StoreError::InvalidCredential)
        ));
        let other = BASE64_STANDARD.encode(r#"{"t":"other","v":{"r":"x","d":"y"}}"#);
        assert!(matches!(decode(&other), Err(StoreError::InvalidCredential)));
    }

    #[test]
    fn header_binds_discharge() {
        let root = macaroon("dashboard.snapcraft.io", "root-id");
        let discharge = macaroon("login.ubuntu.com", "discharge-id");
        let header = auth_header(&encode(&ser(&root), &ser(&discharge)).unwrap()).unwrap();
        let unbound = discharge.serialize(Format::V1).unwrap();
        assert!(header.starts_with("Macaroon root="));
        assert!(
            !header.ends_with(&unbound),
            "discharge must be bound to the root"
        );
    }

    fn with_caveats(location: &str, id: &str, caveats: &[&str]) -> Macaroon {
        let mut m = macaroon(location, id);
        for c in caveats {
            m.add_first_party_caveat((*c).into());
        }
        m
    }

    #[test]
    fn expiry_from_root_and_discharge() {
        let root = with_caveats(
            "myapps.developer.ubuntu.com",
            "root-id",
            &[
                "myapps.developer.ubuntu.com|valid_since|2026-11-10T01:47:03.000180",
                "myapps.developer.ubuntu.com|expires|2027-11-09T18:00:00.000",
            ],
        );
        let discharge = with_caveats(
            "login.ubuntu.com",
            "discharge-id",
            &["login.ubuntu.com|expires|2027-05-01T00:00:00Z"],
        );
        let cred = encode(&ser(&root), &ser(&discharge)).unwrap();
        assert_eq!(
            expiries(&cred).unwrap(),
            vec![
                ("root", "2027-11-09T18:00:00.000Z".to_string()),
                ("discharge", "2027-05-01T00:00:00Z".to_string()),
            ]
        );
    }

    #[test]
    fn expiry_keeps_an_offset() {
        let root = with_caveats("a", "r", &["a|expires|2027-01-01T00:00:00+02:00"]);
        let cred = encode(&ser(&root), &ser(&macaroon("b", "d"))).unwrap();
        assert_eq!(
            expiries(&cred).unwrap(),
            vec![("root", "2027-01-01T00:00:00+02:00".to_string())]
        );
    }

    #[test]
    fn expiry_ignores_other_caveats() {
        let mut root = with_caveats(
            "myapps.developer.ubuntu.com",
            "root-id",
            &[
                "myapps.developer.ubuntu.com|valid_since|2026-11-10T01:47:03",
                "myapps.developer.ubuntu.com|acl|[\"package_push\"]",
                "myapps.developer.ubuntu.com|expires|not a time; rm -rf",
            ],
        );
        let key: macaroon::MacaroonKey = (&[9u8; 32]).into();
        root.add_third_party_caveat("login.ubuntu.com", &key, "tp-id".into());
        let discharge = with_caveats(
            "login.ubuntu.com",
            "discharge-id",
            &["login.ubuntu.com|account|eyJ1c2VybmFtZSI6ICJzZWNyZXQifQ=="],
        );
        let cred = encode(&ser(&root), &ser(&discharge)).unwrap();
        assert!(expiries(&cred).unwrap().is_empty());
    }

    #[test]
    fn expiry_rejects_garbage() {
        assert!(matches!(
            expiries("not base64!"),
            Err(StoreError::InvalidCredential)
        ));
        let other = BASE64_STANDARD.encode(r#"{"t":"other","v":{"r":"x","d":"y"}}"#);
        assert!(matches!(
            expiries(&other),
            Err(StoreError::InvalidCredential)
        ));
        let bad_macaroon = BASE64_STANDARD.encode(r#"{"t":"u1-macaroon","v":{"r":"x","d":"y"}}"#);
        assert!(matches!(
            expiries(&bad_macaroon),
            Err(StoreError::InvalidCredential)
        ));
    }

    #[test]
    fn expiry_output_holds_no_secret() {
        let root = with_caveats(
            "myapps.developer.ubuntu.com",
            "root-id",
            &["myapps.developer.ubuntu.com|expires|2027-11-09T18:00:00"],
        );
        let discharge = with_caveats(
            "login.ubuntu.com",
            "discharge-id",
            &["login.ubuntu.com|account|eyJ1c2VybmFtZSI6ICJzZWNyZXQifQ=="],
        );
        let cred = encode(&ser(&root), &ser(&discharge)).unwrap();
        let printed: String = expiries(&cred)
            .unwrap()
            .iter()
            .map(|(l, t)| format!("{l} {t}\n"))
            .collect();
        let (r, d) = decode(&cred).unwrap();
        let sig = |m: &Macaroon| m.signature().to_vec();
        let sig_b64 = |m: &Macaroon| BASE64_STANDARD.encode(sig(m));
        let sig_b64url = |m: &Macaroon| BASE64_URL_SAFE_NO_PAD.encode(sig(m));
        let sig_hex = |m: &Macaroon| {
            sig(m)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        };
        for secret in [
            cred.clone(),
            r,
            d,
            sig_b64(&root),
            sig_b64(&discharge),
            sig_b64url(&root),
            sig_b64url(&discharge),
            sig_hex(&root),
            sig_hex(&discharge),
            "eyJ1c2VybmFtZSI6".to_string(),
        ] {
            assert!(!printed.contains(&secret), "output leaks {secret}");
        }
        assert_eq!(printed, "root 2027-11-09T18:00:00Z\n");
    }
}
