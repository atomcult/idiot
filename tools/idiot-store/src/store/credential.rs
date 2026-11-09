//! The credential format `snapcraft export-login` writes:
//! `base64({"t":"u1-macaroon","v":{"r":<root>,"d":<discharge>}})`, both
//! macaroons in V1 serialisation, the discharge not yet bound.

use crate::error::StoreError;
use base64::prelude::*;
use macaroon::{Format, Macaroon};
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
}
