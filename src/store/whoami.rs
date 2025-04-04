use anyhow::Result;
use macaroon::{Caveat, Macaroon};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio;

const ENDPOINT: &'static str = "https://dashboard.snapcraft.io/api/v2/tokens/whoami";
const MACAROON: &'static str = "";

#[derive(Debug, Deserialize, Serialize)]
struct StoreMacaroon {
    // type
    t: String,
    v: MacaroonPair,
}

#[derive(Debug, Deserialize, Serialize)]
struct MacaroonPair {
    // root macaroon
    r: String,

    // discharge macaroon
    d: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::ClientBuilder::new()
        .user_agent("snapcraft/8.6.3 ubuntu/24.04 (x86_64)")
        .build()?;

    let mac_bytes = base64::decode(MACAROON)?;
    let mac = String::from_utf8(mac_bytes)?;

    let store_mac: StoreMacaroon = serde_json::from_str(&mac)?;

    let root_mac = Macaroon::deserialize(&store_mac.v.r)?;
    let mut discharge_mac = Macaroon::deserialize(&store_mac.v.d)?;

    root_mac.bind(&mut discharge_mac);
    let discharge_mac_str = discharge_mac.serialize(macaroon::Format::V1)?;

    let auth_str = format!(
        "Macaroon root={}, discharge={}",
        &store_mac.v.r, &discharge_mac_str
    );

    dbg!(&auth_str);

    let resp = client
        .get(ENDPOINT)
        .header("Accept", "*/*")
        .header("Connection", "keep-alive")
        .header("Authorization", auth_str)
        .send()
        .await?;

    dbg!(resp.json::<Value>().await?);
    Ok(())
}
