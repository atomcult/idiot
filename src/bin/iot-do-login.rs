use anyhow::Result;
use macaroon::{Caveat, Macaroon};
use reqwest;
use serde::Serialize;
use serde_json::Value;
use tokio;

// const ENDPOINT: &'static str = "http://127.0.0.1:4444/dev/api/acl/";
const ENDPOINT: &'static str = "https://dashboard.snapcraft.io/dev/api/acl/";

#[derive(Serialize)]
struct Package {
    r#type: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct StoreMacaroon {
    // type
    t: String,
    v: MacaroonPair,
}

#[derive(Debug, Serialize)]
struct MacaroonPair {
    // root macaroon
    r: String,

    // discharge macaroon
    d: String,
}

#[derive(Serialize)]
struct MacaroonRequest {
    permissions: Vec<String>,
    description: String,
    expires: String,

    #[serde(skip_serializing_if = "is_empty")]
    packages: Vec<Package>,

    #[serde(skip_serializing_if = "is_empty")]
    channels: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LoginRequest {
    email: String,
    password: String,
    caveat_id: String,

    #[serde(skip_serializing_if = "is_none")]
    otp: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::ClientBuilder::new()
        .user_agent("snapcraft/8.6.3 ubuntu/24.04 (x86_64)")
        .build()?;

    let payload = MacaroonRequest {
        permissions: vec![String::from("package_access")],
        description: String::from("snapcraft@ubuntu"),
        expires: String::from("2026-02-22T17:47:58+00:00"),
        packages: Vec::new(),
        channels: Vec::new(),
    };

    let resp = client
        .post(ENDPOINT)
        .header("Accept", "application/json")
        .header("Connection", "keep-alive")
        .json(&payload)
        .send()
        .await?;

    let json = resp.json::<Value>().await?;

    let macaroon_str = json
        .get("macaroon")
        .ok_or(anyhow::Error::msg("oops"))?
        .as_str()
        .unwrap();

    let macaroon = Macaroon::deserialize(macaroon_str)?;

    let mut caveat_id = String::new();
    for caveat in macaroon.third_party_caveats() {
        let caveat = match caveat {
            Caveat::ThirdParty(y) => y,
            // FIXME: make this error message better
            _ => continue,
        };

        if &caveat.location() == "login.ubuntu.com" {
            let caveat_id_bin = base64::decode(caveat.id().to_string())?;
            caveat_id = String::from_utf8(caveat_id_bin)?;
            break;
        }
    }

    let payload = LoginRequest {
        email: String::from("ce-team-test@canonical.com"),
        password: String::from("TheLastThingThatHarryToldSally"),
        caveat_id,
        otp: None,
    };

    let resp = client
        .post("https://login.ubuntu.com/api/v2/tokens/discharge")
        .header("Accept", "application/json")
        .header("Connection", "keep-alive")
        .json(&payload)
        .send()
        .await?;

    let json = resp.json::<Value>().await?;
    let discharge_macaroon_str = json["discharge_macaroon"]
        .as_str()
        .ok_or(anyhow::Error::msg("failed to receive discharge macaroon"))?;

    let discharge_macaroon = Macaroon::deserialize(discharge_macaroon_str)?;

    let store_macaroon = serde_json::to_string(&StoreMacaroon {
        t: String::from("u1-macaroon"),
        v: MacaroonPair {
            r: macaroon.serialize(macaroon::Format::V1)?,
            d: discharge_macaroon.serialize(macaroon::Format::V1)?,
        },
    })?;

    let store_macaroon_b64 = base64::encode(&store_macaroon);
    println!("{}", store_macaroon_b64);

    Ok(())
}

fn is_empty<T>(v: &Vec<T>) -> bool
where
    T:,
{
    v.is_empty()
}

fn is_none<T>(o: &Option<T>) -> bool
where
    T:,
{
    o.is_none()
}
