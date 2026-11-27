//! Read-only store queries for `idiot store`: the account's stores and
//! snaps, a store's snaps (admins), a snap's channel map, and whether a
//! revision can be downloaded. Each returns plain records; the command
//! prints them as tab-separated lines for sh to format.
//!
//! Authorised calls send the header snapd builds (`credential::auth_header`)
//! and, when the store answers `macaroon-needs-refresh`, refresh the
//! discharge with Ubuntu SSO and retry once. The fresh discharge is kept in
//! memory only, as craft-store does with credentials from the environment.

use super::{credential, error_message, USER_AGENT};
use crate::error::StoreError;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use serde_json::{json, Value};

const DASHBOARD: &str = "https://dashboard.snapcraft.io";
pub(crate) const API: &str = "https://api.snapcraft.io";
const REFRESH_ENDPOINT: &str = "https://login.ubuntu.com/api/v2/tokens/refresh";
const INFO_FIELDS: &str = "download,revision,version,created-at,architectures";

/// A credential for a run of queries; refreshed in place when needed.
pub struct Auth {
    pub(crate) credential: String,
}

impl Auth {
    pub fn new(credential: &str) -> Self {
        Auth {
            credential: credential.trim().to_string(),
        }
    }
}

/// A response's status, body and whether it asks for a refresh; redirects
/// are never followed.
pub struct Reply {
    pub status: StatusCode,
    pub body: String,
    pub wants_refresh: bool,
}

impl Reply {
    pub(crate) fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
}

pub(crate) fn client() -> Result<Client, StoreError> {
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .redirect(Policy::none())
        .build()?)
}

/// The error codes in a store error body (`error_list` or `error-list`).
pub(crate) fn error_codes(body: &Value) -> Vec<String> {
    ["error_list", "error-list"]
        .iter()
        .filter_map(|k| body.get(k).and_then(Value::as_array))
        .flatten()
        .filter_map(|e| e.get("code").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

/// Whether the store wants the discharge refreshed: the dashboard says so
/// in the error body, downloads in `WWW-Authenticate` (as snapd reads it).
pub fn needs_refresh(body: &Value) -> bool {
    error_codes(body)
        .iter()
        .any(|c| c == "macaroon-needs-refresh")
}

fn header_wants_refresh(resp: &reqwest::blocking::Response) -> bool {
    resp.headers()
        .get(reqwest::header::WWW_AUTHENTICATE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("needs_refresh=1"))
}

/// A response read into a Reply. Only for answers whose body is small (API
/// responses and error bodies), never a snap.
pub(crate) fn reply(resp: reqwest::blocking::Response) -> Reply {
    let status = resp.status();
    let header = header_wants_refresh(&resp);
    let body = resp.text().unwrap_or_default();
    let wants_refresh = status == StatusCode::UNAUTHORIZED
        && (header || needs_refresh(&serde_json::from_str(&body).unwrap_or(Value::Null)));
    Reply {
        status,
        body,
        wants_refresh,
    }
}

/// Sends `build()` as is, or authorised with `auth`, refreshing the
/// discharge and retrying once when the store asks for it.
pub(crate) fn send(
    client: &Client,
    auth: Option<&mut Auth>,
    build: impl Fn() -> RequestBuilder,
) -> Result<Reply, StoreError> {
    let attempt = |cred: Option<&str>| -> Result<Reply, StoreError> {
        let mut rq = build();
        if let Some(c) = cred {
            rq = rq.header("Authorization", credential::auth_header(c)?);
        }
        Ok(reply(rq.send()?))
    };
    let Some(auth) = auth else {
        return attempt(None);
    };
    let first = attempt(Some(&auth.credential))?;
    if !first.wants_refresh {
        return Ok(first);
    }
    auth.credential = refresh(client, &auth.credential)?;
    attempt(Some(&auth.credential))
}

/// The credential with a fresh discharge from Ubuntu SSO.
pub(crate) fn refresh(client: &Client, cred: &str) -> Result<String, StoreError> {
    let (_, discharge) = credential::decode(cred)?;
    let resp = client
        .post(REFRESH_ENDPOINT)
        .header("Accept", "application/json")
        .json(&json!({ "discharge_macaroon": discharge }))
        .send()?;
    let status = resp.status();
    let body: Value = resp.json().unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(StoreError::Rejected);
    }
    let fresh = body
        .get("discharge_macaroon")
        .and_then(Value::as_str)
        .ok_or_else(|| StoreError::Store("no discharge in SSO's refresh answer".into()))?;
    credential::with_discharge(cred, fresh)
}

pub(crate) fn rejected_or(reply: &Reply) -> StoreError {
    match reply.status {
        StatusCode::UNAUTHORIZED => StoreError::Rejected,
        _ => StoreError::Store(format!(
            "{}: {}",
            reply.status,
            error_message(&reply.json())
        )),
    }
}

/// A store the account belongs to.
#[derive(Debug, PartialEq)]
pub struct StoreRole {
    pub id: String,
    pub name: String,
    pub roles: Vec<String>,
}

/// A snap the account publishes or collaborates on (default series).
#[derive(Debug, PartialEq)]
pub struct AccountSnap {
    pub name: String,
    pub store: String,
    pub private: bool,
    /// The publisher's username.
    pub publisher: String,
}

pub fn account(auth: &mut Auth) -> Result<(Vec<StoreRole>, Vec<AccountSnap>), StoreError> {
    let c = client()?;
    let reply = send(&c, Some(auth), || {
        c.get(format!("{DASHBOARD}/dev/api/account"))
            .header("Accept", "application/json")
    })?;
    if !reply.status.is_success() {
        return Err(rejected_or(&reply));
    }
    parse_account(&reply.json())
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn text(v: &Value, k: &str) -> String {
    v.get(k)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn parse_account(json: &Value) -> Result<(Vec<StoreRole>, Vec<AccountSnap>), StoreError> {
    let stores = json
        .get("stores")
        .and_then(Value::as_array)
        .ok_or_else(|| StoreError::Store("unexpected account response".into()))?
        .iter()
        .map(|s| StoreRole {
            id: text(s, "id"),
            name: text(s, "name"),
            roles: strings(&s["roles"]),
        })
        .collect();
    let mut snaps: Vec<AccountSnap> = json["snaps"]["16"]
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(name, info)| AccountSnap {
                    name: name.clone(),
                    store: text(info, "store"),
                    private: info["private"].as_bool().unwrap_or(false),
                    publisher: text(&info["publisher"], "username"),
                })
                .collect()
        })
        .unwrap_or_default();
    snaps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((stores, snaps))
}

/// A snap in a brand store's list (store admins).
#[derive(Debug, PartialEq)]
pub struct StoreSnap {
    pub name: String,
    pub owner: String,
    pub private: bool,
}

/// Why a store's snap list was refused.
#[derive(Debug, PartialEq)]
pub enum Refusal {
    /// The credential lacks this permission.
    Permission(String),
    /// Not an admin of the store, or no such store (the store can't say).
    NotAdmin,
}

pub fn store_snaps(
    auth: &mut Auth,
    store: &str,
) -> Result<Result<Vec<StoreSnap>, Refusal>, StoreError> {
    let c = client()?;
    let reply = send(&c, Some(auth), || {
        c.get(format!("{DASHBOARD}/api/v2/stores/{store}/snaps"))
            .header("Accept", "application/json")
    })?;
    parse_store_snaps(&reply)
}

pub fn parse_store_snaps(reply: &Reply) -> Result<Result<Vec<StoreSnap>, Refusal>, StoreError> {
    let json = reply.json();
    match reply.status {
        s if s.is_success() => {}
        StatusCode::FORBIDDEN
            if error_codes(&json)
                .iter()
                .any(|c| c == "macaroon-permission-required") =>
        {
            let perm = ["error-list", "error_list"]
                .iter()
                .filter_map(|k| json[k].as_array())
                .flatten()
                .find_map(|e| e["extra"]["permission"].as_str())
                .unwrap_or("store_admin");
            return Ok(Err(Refusal::Permission(perm.to_string())));
        }
        StatusCode::NOT_FOUND | StatusCode::FORBIDDEN => return Ok(Err(Refusal::NotAdmin)),
        _ => return Err(rejected_or(reply)),
    }
    let mut snaps: Vec<StoreSnap> = json["snaps"]
        .as_array()
        .ok_or_else(|| StoreError::Store("unexpected store snaps response".into()))?
        .iter()
        .map(|s| StoreSnap {
            name: text(s, "name"),
            owner: s["users"]
                .as_array()
                .and_then(|u| {
                    u.iter()
                        .find(|u| strings(&u["roles"]).iter().any(|r| r == "owner"))
                        .or_else(|| u.first())
                })
                .map(|u| text(u, "username"))
                .unwrap_or_default(),
            private: s["private"].as_bool().unwrap_or(false),
        })
        .collect();
    snaps.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Ok(snaps))
}

/// One channel and architecture in a snap's channel map.
#[derive(Debug, PartialEq, Clone)]
pub struct Channel {
    /// `track/risk`, or `track/risk/branch`.
    pub name: String,
    pub arch: String,
    pub revision: u64,
    pub version: String,
    pub released_at: String,
    pub url: String,
    /// The revision's SHA3-384, hex, and size in bytes.
    pub sha3_384: String,
    pub size: u64,
}

pub(crate) fn info_request(c: &Client, snap: &str, store: Option<&str>) -> RequestBuilder {
    let mut rq = c
        .get(format!("{API}/v2/snaps/info/{snap}"))
        .query(&[("fields", INFO_FIELDS)])
        .header("Snap-Device-Series", "16")
        .header("Accept", "application/json");
    if let Some(id) = store {
        rq = rq.header("Snap-Device-Store", id);
    }
    rq
}

/// The snap's channel map as `auth` (or anyone, without) sees it; None when
/// the store doesn't show the snap.
pub fn info(
    auth: Option<&mut Auth>,
    snap: &str,
    store: Option<&str>,
) -> Result<Option<Vec<Channel>>, StoreError> {
    let c = client()?;
    let reply = send(&c, auth, || info_request(&c, snap, store))?;
    match reply.status {
        StatusCode::NOT_FOUND => Ok(None),
        s if s.is_success() => Ok(Some(parse_info(&reply.json()))),
        _ => Err(rejected_or(&reply)),
    }
}

pub fn parse_info(json: &Value) -> Vec<Channel> {
    let mut out: Vec<Channel> = json["channel-map"]
        .as_array()
        .map(|m| {
            m.iter()
                .map(|e| {
                    let ch = &e["channel"];
                    let mut name = format!("{}/{}", text(ch, "track"), text(ch, "risk"));
                    if let Some(b) = ch["branch"].as_str().filter(|b| !b.is_empty()) {
                        name = format!("{name}/{b}");
                    }
                    Channel {
                        name,
                        arch: text(ch, "architecture"),
                        revision: e["revision"].as_u64().unwrap_or(0),
                        version: text(e, "version"),
                        released_at: text(ch, "released-at"),
                        url: text(&e["download"], "url"),
                        sha3_384: text(&e["download"], "sha3-384"),
                        size: e["download"]["size"].as_u64().unwrap_or(0),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| (&a.name, &a.arch).cmp(&(&b.name, &b.arch)));
    out
}

const RISKS: [&str; 4] = ["stable", "candidate", "beta", "edge"];

/// A channel as snapd reads it: a first segment that is a risk means the
/// `latest` track (`stable` -> `latest/stable`, `edge/fix` ->
/// `latest/edge/fix`), a track alone means its `stable` risk; `2.0/edge`
/// and `latest/edge/fix` as given.
pub fn normalise_channel(channel: &str) -> String {
    let first = channel.split('/').next().unwrap_or("");
    if RISKS.contains(&first) {
        format!("latest/{channel}")
    } else if !channel.contains('/') {
        format!("{channel}/stable")
    } else {
        channel.to_string()
    }
}

/// What a download attempt got: the status only (a redirect to the CDN is
/// the store allowing it). The body is never read.
pub fn download_status(auth: Option<&mut Auth>, url: &str) -> Result<StatusCode, StoreError> {
    let c = client()?;
    let attempt = |cred: Option<&str>| -> Result<reqwest::blocking::Response, StoreError> {
        let mut rq = c.get(url);
        if let Some(cr) = cred {
            rq = rq.header("Authorization", credential::auth_header(cr)?);
        }
        Ok(rq.send()?)
    };
    let Some(auth) = auth else {
        return Ok(attempt(None)?.status());
    };
    let resp = attempt(Some(&auth.credential))?;
    if resp.status() != StatusCode::UNAUTHORIZED {
        return Ok(resp.status());
    }
    // a 401's body is a short error, never the snap: safe to read
    if !reply(resp).wants_refresh {
        return Ok(StatusCode::UNAUTHORIZED);
    }
    auth.credential = refresh(&c, &auth.credential)?;
    Ok(attempt(Some(&auth.credential))?.status())
}

/// What `idiot store check` needs to know about one snap, revision and
/// download, as lines: `info anon|auth <status>`, then `release found
/// <channel> <arch> <revision>` or `release missing <channel> <arch>`, then
/// `download anon|auth <status>`. `auth` lines only with a credential.
pub fn probe(
    mut auth: Option<&mut Auth>,
    snap: &str,
    store: Option<&str>,
    channel: &str,
    arch: &str,
) -> Result<Vec<String>, StoreError> {
    let c = client()?;
    let mut lines = Vec::new();
    let anon = send(&c, None, || info_request(&c, snap, store))?;
    lines.push(format!("info\tanon\t{}", anon.status.as_u16()));
    let mut seen = anon;
    if let Some(a) = auth.as_deref_mut() {
        let authed = send(&c, Some(a), || info_request(&c, snap, store))?;
        lines.push(format!("info\tauth\t{}", authed.status.as_u16()));
        if authed.status.is_success() {
            seen = authed;
        }
    }
    if !seen.status.is_success() {
        return Ok(lines);
    }
    let channel = normalise_channel(channel);
    let map = parse_info(&seen.json());
    let Some(entry) = map.iter().find(|e| e.name == channel && e.arch == arch) else {
        lines.push(format!("release\tmissing\t{channel}\t{arch}"));
        return Ok(lines);
    };
    lines.push(format!(
        "release\tfound\t{channel}\t{arch}\t{}",
        entry.revision
    ));
    lines.push(format!(
        "download\tanon\t{}",
        download_status(None, &entry.url)?.as_u16()
    ));
    if let Some(a) = auth {
        lines.push(format!(
            "download\tauth\t{}",
            download_status(Some(a), &entry.url)?.as_u16()
        ));
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/src/testdata/store/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn fake(status: u16, name: &str) -> Reply {
        Reply {
            status: StatusCode::from_u16(status).unwrap(),
            body: fixture(name).to_string(),
            wants_refresh: false,
        }
    }

    #[test]
    fn account_stores_and_snaps() {
        let (stores, snaps) = parse_account(&fixture("account.json")).unwrap();
        assert_eq!(stores.len(), 3);
        assert_eq!(stores[1].id, "abc123StoreIdXXXXXXX");
        assert_eq!(stores[1].name, "Example Team Store");
        assert_eq!(stores[1].roles, ["admin", "access", "review", "view"]);
        assert_eq!(
            snaps,
            [
                AccountSnap {
                    name: "hello-global".into(),
                    store: "Global".into(),
                    private: false,
                    publisher: "someone".into()
                },
                AccountSnap {
                    name: "hello-team".into(),
                    store: "Example Team Store".into(),
                    private: true,
                    publisher: "test-user".into()
                },
            ]
        );
    }

    #[test]
    fn store_snaps_listed() {
        let snaps = parse_store_snaps(&fake(200, "store-snaps.json"))
            .unwrap()
            .unwrap();
        assert_eq!(
            snaps[0],
            StoreSnap {
                name: "core".into(),
                owner: "publisher".into(),
                private: false
            }
        );
        assert_eq!(
            snaps[1],
            StoreSnap {
                name: "hello-team".into(),
                owner: "test-user".into(),
                private: true
            }
        );
    }

    #[test]
    fn store_snaps_refusals() {
        assert_eq!(
            parse_store_snaps(&fake(403, "store-snaps-403.json")).unwrap(),
            Err(Refusal::Permission("store_admin".into()))
        );
        let not_found = Reply {
            status: StatusCode::NOT_FOUND,
            body: "{}".into(),
            wants_refresh: false,
        };
        assert_eq!(
            parse_store_snaps(&not_found).unwrap(),
            Err(Refusal::NotAdmin)
        );
    }

    #[test]
    fn info_channel_map() {
        let map = parse_info(&fixture("info.json"));
        assert_eq!(map.len(), 2);
        assert_eq!(map[0].name, "latest/edge");
        assert_eq!(map[0].arch, "arm64");
        assert_eq!(map[0].revision, 4);
        assert_eq!(map[1].name, "latest/stable");
        assert!(map[1].url.ends_with("_3.snap"));
        let map = parse_info(&fixture("info-hello-world.json"));
        let e = map
            .iter()
            .find(|e| e.name == "latest/stable" && e.arch == "amd64")
            .unwrap();
        assert_eq!(e.revision, 29);
        assert_eq!(e.size, 20480);
        assert!(e.sha3_384.starts_with("b07bdb78e762c2e6"));
        assert_eq!(
            text(&fixture("info-hello-world.json"), "snap-id"),
            "buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ"
        );
    }

    #[test]
    fn refresh_trigger() {
        assert!(needs_refresh(&fixture("needs-refresh.json")));
        assert!(!needs_refresh(&fixture("store-snaps-403.json")));
        assert!(!needs_refresh(&Value::Null));
    }

    /// A one-request HTTP server answering 302: the probe must report the
    /// redirect, not follow it.
    #[test]
    fn download_does_not_follow_redirects() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            s.write_all(
                b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/never\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        });
        let status = download_status(None, &format!("http://127.0.0.1:{port}/x.snap")).unwrap();
        assert_eq!(status, StatusCode::FOUND);
        server.join().unwrap();
    }

    /// A download 401 asking for a refresh in WWW-Authenticate (as the store
    /// sends it) is recognised.
    #[test]
    fn refresh_from_header() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            s.write_all(b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Macaroon needs_refresh=1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let resp = client()
            .unwrap()
            .get(format!("http://127.0.0.1:{port}/x"))
            .send()
            .unwrap();
        assert!(reply(resp).wants_refresh);
        server.join().unwrap();
    }

    #[test]
    fn channels_normalised() {
        assert_eq!(normalise_channel("stable"), "latest/stable");
        assert_eq!(normalise_channel("2.0/edge"), "2.0/edge");
        assert_eq!(normalise_channel("latest/edge/fix"), "latest/edge/fix");
        assert_eq!(normalise_channel("edge/fix"), "latest/edge/fix");
        assert_eq!(normalise_channel("beta"), "latest/beta");
        assert_eq!(normalise_channel("2.0"), "2.0/stable");
    }
}
