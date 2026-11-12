//! The credential format `snapcraft export-login` writes:
//! `base64({"t":"u1-macaroon","v":{"r":<root>,"d":<discharge>}})`, both
//! macaroons in V1 serialisation, the discharge not yet bound.

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

/// Encodes a root and discharge macaroon as a credential.
pub fn encode(root: &Macaroon, discharge: &Macaroon) -> Result<String, StoreError> {
    let ser = |m: &Macaroon| {
        m.serialize(Format::V1)
            .map_err(|e| StoreError::Store(format!("macaroon: {e:?}")))
    };
    let json = serde_json::to_string(&Credential {
        t: TOKEN_TYPE.to_string(),
        v: Pair {
            r: ser(root)?,
            d: ser(discharge)?,
        },
    })
    .map_err(|e| StoreError::Store(e.to_string()))?;
    Ok(BASE64_STANDARD.encode(json))
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

/// The `Authorization` header value the store expects for a credential:
/// the root macaroon and the discharge bound to it.
pub fn auth_header(credential: &str) -> Result<String, StoreError> {
    let (root_str, discharge_str) = decode(credential)?;
    let root = Macaroon::deserialize(&root_str).map_err(|_| StoreError::InvalidCredential)?;
    let mut discharge =
        Macaroon::deserialize(&discharge_str).map_err(|_| StoreError::InvalidCredential)?;
    root.bind(&mut discharge);
    let bound = discharge
        .serialize(Format::V1)
        .map_err(|_| StoreError::InvalidCredential)?;
    Ok(format!("Macaroon root={root_str}, discharge={bound}"))
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

    #[test]
    fn round_trip() {
        let root = macaroon("dashboard.snapcraft.io", "root-id");
        let discharge = macaroon("login.ubuntu.com", "discharge-id");
        let cred = encode(&root, &discharge).unwrap();

        let (r, d) = decode(&cred).unwrap();
        assert_eq!(Macaroon::deserialize(&r).unwrap(), root);
        assert_eq!(Macaroon::deserialize(&d).unwrap(), discharge);
        // trailing newline from a file is fine
        assert!(decode(&format!("{cred}\n")).is_ok());
    }

    #[test]
    fn snapcraft_layout() {
        let cred = encode(&macaroon("a", "r"), &macaroon("b", "d")).unwrap();
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
        let header = auth_header(&encode(&root, &discharge).unwrap()).unwrap();
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
        let cred = encode(&root, &discharge).unwrap();
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
        let cred = encode(&root, &macaroon("b", "d")).unwrap();
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
        let cred = encode(&root, &discharge).unwrap();
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
        let cred = encode(&root, &discharge).unwrap();
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
