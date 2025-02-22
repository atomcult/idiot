use anyhow::Result;
use macaroon::{Caveat, Macaroon};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio;

const ENDPOINT: &'static str = "https://dashboard.snapcraft.io/api/v2/tokens/whoami";
const MACAROON: &'static str = "eyJ0IjoidTEtbWFjYXJvb24iLCJ2Ijp7InIiOiJNREF5T1d4dlkyRjBhVzl1SUcxNVlYQndjeTVrWlhabGJHOXdaWEl1ZFdKMWJuUjFMbU52YlFvd01ERTJhV1JsYm5ScFptbGxjaUJOZVVGd2NITUtNREEwWW1OcFpDQnRlV0Z3Y0hNdVpHVjJaV3h2Y0dWeUxuVmlkVzUwZFM1amIyMThkbUZzYVdSZmMybHVZMlY4TWpBeU5TMHdNaTB5TWxReE9Ub3pNam96TkM0eE16SXlPVGtLTURFM1pHTnBaQ0I3SW5abGNuTnBiMjRpT2lBeExDQWljMlZqY21WMElqb2dJbVV6YUhZcldqZFZNVUpUZGpSb1QzWjBRMmd4UlRjck1VZGFlRmRUWmpremVXTmpXbFptTWxjM1FqQTRlRTVCVFU1R1JrRm1ia0ZvYVV0TFRrUTVOa2gwWWpNdlpGaHNUemhzVEU4ME1tWlJNWEZPTjBReU1tRTRNemM1TkN0dWVrOVNVVWxsTDNjMFNWUkViVTFOUWt4M1YybzBVQ3NyWjBwSU0zQkNRakZGVGxWblNDOUxTMWs1Y0RGRFNYSjRNVTFqTUU1WFEwNHpUM054YkhJMVJUa3Zia0pwTVZSNU4yUjZWRXBCUVhwMWFubDJOVnBMV25GeGVHTnpaazlHYjI1b1VrdHhOR0p3VDNoUlQwTjRjamtyUmxreWMyWllTMjR4UjJ4NWJVcExjMEphY25wdVVsbzFWMmxPTVVsV1VGRm5SVkpxYTBrMllYSlVXaXRVU3paeWFYcHpaR0p1U2tRMWVWbE1RMjVYYTFCRGMxWnVZa0pET0dOSlZHUXpZbk5QTjFoMFRXODFjR3BDVkRsUVdXOTBjbEpLY21kakt6aHpTelpyWWpOMGJDc3JhRGwxYzFaQ1dEYzVTR2RqV214V1lsbDRVVDA5SW4wS01EQTFNWFpwWkNETWVPc3Nfb2o3bEtxUkxnT2NiRUNHSGFHWVNEOFhTbG5OM1RiNW5xSmZ6Ny1kZmhmQkpTU2ZVd2VXRnVZQjA5MUZFVWRhMU5hR09NeUlIQjFDeVBtc2lDWHhMMHNXTVVBS01EQXhPR05zSUd4dloybHVMblZpZFc1MGRTNWpiMjBLTURBelltTnBaQ0J0ZVdGd2NITXVaR1YyWld4dmNHVnlMblZpZFc1MGRTNWpiMjE4WVdOc2ZGc2ljR0ZqYTJGblpWOWhZMk5sYzNNaVhRb3dNRFEzWTJsa0lHMTVZWEJ3Y3k1a1pYWmxiRzl3WlhJdWRXSjFiblIxTG1OdmJYeGxlSEJwY21WemZESXdNall0TURJdE1qSlVNVGM2TkRjNk5UZ3VNREF3TVRReENqQXdNbVp6YVdkdVlYUjFjbVVndTVtbjB5N3pJOEZkRDFFQ0kwbEtBbnpkWmpxdkZkQVBrcFQzNUF0MlFWQUsiLCJkIjoiTURBeFpXeHZZMkYwYVc5dUlHeHZaMmx1TG5WaWRXNTBkUzVqYjIwS01ERTROR2xrWlc1MGFXWnBaWElnZXlKMlpYSnphVzl1SWpvZ01Td2dJbk5sWTNKbGRDSTZJQ0psTTJoMksxbzNWVEZDVTNZMGFFOTJkRU5vTVVVM0t6RkhXbmhYVTJZNU0zbGpZMXBXWmpKWE4wSXdPSGhPUVUxT1JrWkJabTVCYUdsTFMwNUVPVFpJZEdJekwyUlliRTg0YkV4UE5ESm1VVEZ4VGpkRU1qSmhPRE0zT1RRcmJucFBVbEZKWlM5M05FbFVSRzFOVFVKTWQxZHFORkFySzJkS1NETndRa0l4UlU1VlowZ3ZTMHRaT1hBeFEwbHllREZOWXpCT1YwTk9NMDl6Y1d4eU5VVTVMMjVDYVRGVWVUZGtlbFJLUVVGNmRXcDVkalZhUzFweGNYaGpjMlpQUm05dWFGSkxjVFJpY0U5NFVVOURlSEk1SzBaWk1uTm1XRXR1TVVkc2VXMUtTM05DV25KNmJsSmFOVmRwVGpGSlZsQlJaMFZTYW10Sk5tRnlWRm9yVkVzMmNtbDZjMlJpYmtwRU5YbFpURU51VjJ0UVEzTldibUpDUXpoalNWUmtNMkp6VHpkWWRFMXZOWEJxUWxRNVVGbHZkSEpTU25Kbll5czRjMHMyYTJJemRHd3JLMmc1ZFhOV1FsZzNPVWhuWTFwc1ZtSlplRkU5UFNKOUNqQXdaR1ZqYVdRZ2JHOW5hVzR1ZFdKMWJuUjFMbU52Ylh4aFkyTnZkVzUwZkdWNVNteGlWMFp3WWtOSk5rbERTbXBhVXpFd1dsZEdkRXhZVW14ak0xSkJXVEpHZFdJeU5YQlpNa1p6VEcxT2RtSlRTWE5KUTBwMlkwZFdkV0ZYVVdsUGFVRnBaVlpSTUZWSE5VUmpRMGx6U1VOS2NHTXhPVEphV0Vwd1dtMXNiRnBEU1RaSlNGSjVaRmRWYzBsRFNqRmpNbFo1WW0xR2RGcFRTVFpKUTBwcVdsTXhNRnBYUm5STVdGSnNZek5SYVV4RFFXbGFSMng2WTBkNGFHVlhOV2hpVjFWcFQybEJhVkV3VldkV1IxWm9ZbE5DVlZwWVRqQkpiakE5Q2pBd05EQmphV1FnYkc5bmFXNHVkV0oxYm5SMUxtTnZiWHgyWVd4cFpGOXphVzVqWlh3eU1ESTFMVEF5TFRJeVZERTVPak15T2pNMUxqSTFOek15TWdvd01ETmxZMmxrSUd4dloybHVMblZpZFc1MGRTNWpiMjE4YkdGemRGOWhkWFJvZkRJd01qVXRNREl0TWpKVU1UazZNekk2TXpVdU1qVTNNekl5Q2pBd01tWnphV2R1WVhSMWNtVWc1ZzhXclg0ZTB4eVlLQmk3b0NCcUszNjBWVnlPdEJlYmNzdUxYMUN3SVFrSyJ9fQ==";

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
