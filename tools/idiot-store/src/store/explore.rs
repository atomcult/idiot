//! The queries behind `idiot store browse`: several snaps' channel maps
//! with the fields the explorer filters on, and a snap's revisions and
//! release history from the dashboard. Read-only, like the rest of
//! `query`; sh does all the filtering.
//!
//! Channel maps are fetched up to four at a time, sharing one credential.
//! When the store asks for a fresh discharge, the refresh is serialised
//! behind a mutex and happens at most once per run.

use super::query::{
    self, client, error_codes, parse_info, rejected_or, reply, strings, text, Channel, Reply,
};
use crate::error::StoreError;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::{StatusCode, Url};
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// What `channel-maps` asks v2 info for: the channel map's revisions with
/// what the explorer shows and filters on, and the snap's own details.
const MAP_FIELDS: &str = "architectures,base,confinement,created-at,download,epoch,revision,type,version,publisher,private,title";

/// Requests in flight at once for `channel-maps`.
const CONCURRENCY: usize = 4;

/// Pages of release history read at most (500 releases a page).
const MAX_PAGES: usize = 20;

/// Where the queries go: the store, or a local server in tests.
pub struct Endpoints {
    pub api: String,
    pub dashboard: String,
    pub refresh: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints {
            api: query::API.to_string(),
            dashboard: query::DASHBOARD.to_string(),
            refresh: query::REFRESH_ENDPOINT.to_string(),
        }
    }
}

/// A credential shared by concurrent requests: refreshed once at most.
pub struct SharedAuth {
    credential: Mutex<String>,
    refreshed: Mutex<bool>,
}

impl SharedAuth {
    pub fn new(credential: &str) -> Self {
        SharedAuth {
            credential: Mutex::new(credential.trim().to_string()),
            refreshed: Mutex::new(false),
        }
    }

    fn current(&self) -> String {
        self.credential.lock().unwrap().clone()
    }
}

/// Sends `build()`, authorised with `auth` when given. On a refresh
/// request the first thread to ask refreshes (once per run); the others
/// wait for it and retry with the fresh credential.
fn send_shared(
    c: &Client,
    ep: &Endpoints,
    auth: Option<&SharedAuth>,
    build: impl Fn() -> RequestBuilder,
) -> Result<Reply, StoreError> {
    let attempt = |cred: Option<&str>| -> Result<Reply, StoreError> {
        let mut rq = build();
        if let Some(cr) = cred {
            rq = rq.header("Authorization", super::credential::auth_header(cr)?);
        }
        Ok(reply(rq.send()?))
    };
    let Some(auth) = auth else {
        return attempt(None);
    };
    let used = auth.current();
    let first = attempt(Some(&used))?;
    if !first.wants_refresh {
        return Ok(first);
    }
    let mut refreshed = auth.refreshed.lock().unwrap();
    let now = auth.current();
    if now != used {
        drop(refreshed);
        return attempt(Some(&now));
    }
    if *refreshed {
        // refreshed already and still asked: the caller sees the 401
        return Ok(first);
    }
    let fresh = query::refresh_at(c, &ep.refresh, &used)?;
    *auth.credential.lock().unwrap() = fresh.clone();
    *refreshed = true;
    drop(refreshed);
    attempt(Some(&fresh))
}

/// A snap's own details from v2 info.
#[derive(Debug, PartialEq)]
pub struct SnapMeta {
    pub snap_id: String,
    pub publisher: String,
    pub private: bool,
    pub title: String,
}

/// One snap's answer: its details and channel map, or None when the store
/// doesn't show it.
pub type MapResult = Option<(SnapMeta, Vec<Channel>)>;

pub fn parse_meta(json: &Value) -> SnapMeta {
    let snap = &json["snap"];
    SnapMeta {
        snap_id: text(json, "snap-id"),
        publisher: text(&snap["publisher"], "username"),
        private: snap["private"].as_bool().unwrap_or(false),
        title: text(snap, "title"),
    }
}

fn map_request(c: &Client, ep: &Endpoints, snap: &str, store: Option<&str>) -> RequestBuilder {
    let mut rq = c
        .get(format!("{}/v2/snaps/info/{snap}", ep.api))
        .query(&[("fields", MAP_FIELDS)])
        .header("Snap-Device-Series", "16")
        .header("Accept", "application/json");
    if let Some(id) = store {
        rq = rq.header("Snap-Device-Store", id);
    }
    rq
}

/// The channel maps of `snaps`, in their order, as `auth` (or anyone,
/// without) sees them in `store`.
pub fn channel_maps(
    ep: &Endpoints,
    auth: Option<&SharedAuth>,
    snaps: &[String],
    store: Option<&str>,
) -> Result<Vec<MapResult>, StoreError> {
    let c = client()?;
    let next = AtomicUsize::new(0);
    let results: Vec<Mutex<Option<Result<MapResult, StoreError>>>> =
        snaps.iter().map(|_| Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..CONCURRENCY.min(snaps.len()) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(snap) = snaps.get(i) else {
                    break;
                };
                let got =
                    send_shared(&c, ep, auth, || map_request(&c, ep, snap, store)).and_then(|r| {
                        match r.status {
                            StatusCode::NOT_FOUND => Ok(None),
                            s if s.is_success() => {
                                let json = r.json();
                                Ok(Some((parse_meta(&json), parse_info(&json))))
                            }
                            _ => Err(rejected_or(&r)),
                        }
                    });
                *results[i].lock().unwrap() = Some(got);
            });
        }
    });
    results
        .into_iter()
        .map(|m| m.into_inner().unwrap().expect("every snap was fetched"))
        .collect()
}

/// One uploaded revision, from the dashboard.
#[derive(Debug, PartialEq)]
pub struct Revision {
    pub revision: u64,
    pub version: String,
    pub architectures: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub base: Option<String>,
    pub confinement: String,
    pub grade: String,
    pub epoch: String,
    pub size: u64,
    pub sha3_384: String,
}

/// One entry of the release history: None as the revision closes it.
#[derive(Debug, PartialEq)]
pub struct Release {
    pub channel: String,
    pub arch: String,
    pub revision: Option<u64>,
    pub when: String,
    pub expires: Option<String>,
    pub progressive: Option<f64>,
}

/// Why the history was refused.
#[derive(Debug, PartialEq)]
pub enum HistoryRefusal {
    /// The credential lacks this permission.
    Permission(String),
    /// Neither publisher nor collaborator (or no such snap: the store
    /// won't say).
    NotCollaborator,
    /// No credential to ask with.
    NoCredential,
}

/// A snap's history: every revision and every release.
#[derive(Debug, PartialEq, Default)]
pub struct History {
    pub revisions: Vec<Revision>,
    pub releases: Vec<Release>,
}

/// The first of `keys` that `v` has as a string.
fn either(v: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|k| v[*k].as_str())
        .unwrap_or_default()
        .to_string()
}

fn opt_text(v: &Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty()).map(str::to_owned)
}

/// One page of the dashboard's releases answer, added to `into`.
pub fn parse_releases_page(json: &Value, into: &mut History) {
    for r in json["revisions"].as_array().into_iter().flatten() {
        into.revisions.push(Revision {
            revision: r["revision"].as_u64().unwrap_or(0),
            version: text(r, "version"),
            architectures: strings(&r["architectures"]),
            status: text(r, "status"),
            created_at: either(r, &["created_at", "created-at"]),
            base: opt_text(&r["base"]),
            confinement: text(r, "confinement"),
            grade: text(r, "grade"),
            epoch: query::epoch_string(&r["epoch"]),
            size: r["size"].as_u64().unwrap_or(0),
            sha3_384: either(r, &["sha3-384", "sha3_384"]),
        });
    }
    for r in json["releases"].as_array().into_iter().flatten() {
        let track = text(r, "track");
        let risk = text(r, "risk");
        let mut channel = if track.is_empty() || risk.is_empty() {
            query::normalise_channel(&text(r, "channel"))
        } else {
            format!("{track}/{risk}")
        };
        if let Some(b) = opt_text(&r["branch"]) {
            if !channel.ends_with(&format!("/{b}")) {
                channel = format!("{channel}/{b}");
            }
        }
        let progressive = &r["progressive"];
        into.releases.push(Release {
            channel,
            arch: text(r, "architecture"),
            revision: r["revision"].as_u64(),
            when: text(r, "when"),
            expires: opt_text(&r["expiration-date"]),
            progressive: progressive["percentage"]
                .as_f64()
                .or_else(|| progressive["current-percentage"].as_f64()),
        });
    }
}

/// How the store answered a releases page: Ok(None) for a success.
pub fn releases_refusal(reply: &Reply) -> Result<Option<HistoryRefusal>, StoreError> {
    let json = reply.json();
    match reply.status {
        s if s.is_success() => Ok(None),
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
                .unwrap_or("package_access");
            Ok(Some(HistoryRefusal::Permission(perm.to_string())))
        }
        StatusCode::NOT_FOUND | StatusCode::FORBIDDEN => Ok(Some(HistoryRefusal::NotCollaborator)),
        _ => Err(rejected_or(reply)),
    }
}

/// The page after this one, when `_links.next` names one on the same
/// origin as `current` (so the credential goes nowhere else).
pub fn next_page(json: &Value, current: &Url) -> Option<Url> {
    let next = json["_links"]["next"]
        .get("href")
        .and_then(Value::as_str)
        .or_else(|| json["_links"]["next"].as_str())?;
    let url = current.join(next).ok()?;
    (url.origin() == current.origin()).then_some(url)
}

/// What `releases` got: the history, refused, plus whether a link to
/// another origin or the page cap stopped it early (a note for stderr).
pub struct HistoryAnswer {
    pub history: Result<History, HistoryRefusal>,
    pub stopped: Option<String>,
}

/// Every page of `snap`'s revisions and release history.
pub fn releases(
    ep: &Endpoints,
    auth: Option<&SharedAuth>,
    snap: &str,
) -> Result<HistoryAnswer, StoreError> {
    let Some(auth) = auth else {
        return Ok(HistoryAnswer {
            history: Err(HistoryRefusal::NoCredential),
            stopped: None,
        });
    };
    let c = client()?;
    let mut url = Url::parse(&format!("{}/api/v2/snaps/{snap}/releases", ep.dashboard))
        .map_err(|e| StoreError::Store(e.to_string()))?;
    let mut history = History::default();
    for page in 1..=MAX_PAGES {
        let reply = send_shared(&c, ep, Some(auth), || {
            c.get(url.clone()).header("Accept", "application/json")
        })?;
        if let Some(r) = releases_refusal(&reply)? {
            return Ok(HistoryAnswer {
                history: Err(r),
                stopped: None,
            });
        }
        let json = reply.json();
        parse_releases_page(&json, &mut history);
        let has_next = !json["_links"]["next"].is_null();
        match next_page(&json, &url) {
            Some(n) if page < MAX_PAGES => url = n,
            Some(_) => {
                return Ok(HistoryAnswer {
                    history: Ok(history),
                    stopped: Some(format!(
                        "{snap}: stopped after {MAX_PAGES} pages of history"
                    )),
                })
            }
            None if has_next => {
                return Ok(HistoryAnswer {
                    history: Ok(history),
                    stopped: Some(format!(
                        "{snap}: the next page of history is on another host; not followed"
                    )),
                })
            }
            None => break,
        }
    }
    Ok(HistoryAnswer {
        history: Ok(history),
        stopped: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::credential::{self, tests as cred};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::Arc;

    fn fixture(name: &str) -> Value {
        let path = format!("{}/src/testdata/store/{name}", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn fake(status: u16, body: &str) -> Reply {
        Reply {
            status: StatusCode::from_u16(status).unwrap(),
            body: body.into(),
            wants_refresh: false,
        }
    }

    #[test]
    fn epochs_as_snapd_writes_them() {
        let e = |v: Value| query::epoch_string(&v);
        assert_eq!(e(serde_json::json!({"read": [0], "write": [0]})), "0");
        assert_eq!(e(serde_json::json!({"read": [0, 1], "write": [1]})), "1*");
        assert_eq!(
            e(serde_json::json!({"read": [1, 2, 3], "write": [3]})),
            "read=1,2,3;write=3"
        );
        assert_eq!(e(Value::Null), "");
    }

    #[test]
    fn channel_map_fields() {
        let json = fixture("channel-map.json");
        let meta = parse_meta(&json);
        assert_eq!(
            meta,
            SnapMeta {
                snap_id: "SnapIdCeDummyXXXXXXXXXXXXXXXXXXX".into(),
                publisher: "test-user".into(),
                private: true,
                title: "CE\tdummy".into(),
            }
        );
        let map = parse_info(&json);
        assert_eq!(map.len(), 3);
        let edge = map.iter().find(|c| c.name == "latest/edge").unwrap();
        assert_eq!(
            (edge.name.as_str(), edge.arch.as_str(), edge.revision),
            ("latest/edge", "arm64", 4)
        );
        assert_eq!(edge.kind, "app");
        assert_eq!(edge.base.as_deref(), Some("core24"));
        assert_eq!(edge.confinement, "strict");
        assert_eq!(edge.epoch, "1*");
        assert_eq!(edge.size, 8192);
        assert_eq!(edge.sha3_384, "ab".repeat(48));
        assert_eq!(edge.created_at, "2026-10-03T09:00:00.000000+00:00");
        // a base snap has no base
        let base = fixture("channel-map-base.json");
        let m = parse_info(&base);
        assert_eq!(m[0].base, None);
        assert_eq!(m[0].kind, "base");
        assert_eq!(m[0].epoch, "0");
        assert_eq!(parse_meta(&base).publisher, "canonical");
    }

    #[test]
    fn releases_history() {
        let mut h = History::default();
        parse_releases_page(&fixture("releases.json"), &mut h);
        assert_eq!(h.revisions.len(), 4);
        let unreleased = h.revisions.iter().find(|r| r.revision == 5).unwrap();
        assert_eq!(unreleased.status, "Published");
        assert_eq!(unreleased.created_at, "2026-10-05T09:00:00Z");
        assert_eq!(unreleased.sha3_384, "cd".repeat(48));
        let rejected = h.revisions.iter().find(|r| r.revision == 6).unwrap();
        assert_eq!(rejected.status, "Rejected");
        // the other spelling of the hash and the time
        assert_eq!(rejected.sha3_384, "ef".repeat(48));
        assert_eq!(rejected.created_at, "2026-10-06T09:00:00Z");
        assert_eq!(rejected.epoch, "1*");
        assert_eq!(rejected.base, None);
        assert!(!h.releases.is_empty());
        let closed = h.releases.iter().find(|r| r.revision.is_none()).unwrap();
        assert_eq!(closed.channel, "latest/beta");
        let progressive = h.releases.iter().find(|r| r.progressive.is_some()).unwrap();
        assert_eq!(progressive.progressive, Some(30.0));
        assert_eq!(progressive.revision, Some(4));
        let branch = h
            .releases
            .iter()
            .find(|r| r.channel == "latest/edge/fix-1")
            .unwrap();
        assert_eq!(branch.expires.as_deref(), Some("2026-11-03T10:00:00Z"));
    }

    #[test]
    fn releases_refused() {
        let perm = r#"{"error-list": [{"code": "macaroon-permission-required", "extra": {"permission": "package_access"}, "message": "Missing permission"}]}"#;
        assert_eq!(
            releases_refusal(&fake(403, perm)).unwrap(),
            Some(HistoryRefusal::Permission("package_access".into()))
        );
        assert_eq!(
            releases_refusal(&fake(404, "{}")).unwrap(),
            Some(HistoryRefusal::NotCollaborator)
        );
        assert_eq!(releases_refusal(&fake(200, "{}")).unwrap(), None);
        assert!(releases_refusal(&fake(500, "{}")).is_err());
    }

    #[test]
    fn next_links_stay_on_the_dashboard() {
        let here = Url::parse("https://dashboard.snapcraft.io/api/v2/snaps/x/releases").unwrap();
        let rel =
            serde_json::json!({"_links": {"next": {"href": "/api/v2/snaps/x/releases?page=2"}}});
        assert_eq!(
            next_page(&rel, &here).unwrap().as_str(),
            "https://dashboard.snapcraft.io/api/v2/snaps/x/releases?page=2"
        );
        let abs = serde_json::json!({"_links": {"next": "https://dashboard.snapcraft.io/api/v2/snaps/x/releases?page=3"}});
        assert!(next_page(&abs, &here).is_some());
        let away =
            serde_json::json!({"_links": {"next": {"href": "https://elsewhere.example/steal"}}});
        assert_eq!(next_page(&away, &here), None);
        assert_eq!(next_page(&serde_json::json!({}), &here), None);
    }

    /// A credential the refresh code can decode and re-encode.
    fn test_credential() -> String {
        let root = cred::macaroon("dashboard.snapcraft.io", "root-id");
        let discharge = cred::macaroon("login.ubuntu.com", "discharge-id");
        credential::encode(&cred::ser(&root), &cred::ser(&discharge)).unwrap()
    }

    /// Reads one HTTP request (head and body); returns its request line.
    fn read_request(s: &mut std::net::TcpStream) -> String {
        let mut r = BufReader::new(s.try_clone().unwrap());
        let mut first = String::new();
        r.read_line(&mut first).unwrap();
        let mut len = 0;
        loop {
            let mut h = String::new();
            r.read_line(&mut h).unwrap();
            if h.trim().is_empty() {
                break;
            }
            if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                len = v.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; len];
        r.read_exact(&mut body).unwrap();
        first
    }

    fn respond(s: &mut std::net::TcpStream, status: &str, body: &str) {
        let _ = write!(
            s,
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    }

    /// A local server: info requests get a 401 asking for a refresh until
    /// the discharge has been refreshed, then a channel map; next links
    /// on page 1 of releases point to another host. Counts refreshes.
    fn server(requests: usize) -> (Endpoints, Arc<AtomicUsize>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
        let refreshes = Arc::new(AtomicUsize::new(0));
        let count = refreshes.clone();
        let fresh = cred::ser(&cred::macaroon("login.ubuntu.com", "fresh-id"));
        let map = fixture("channel-map.json").to_string();
        let releases = fixture("releases.json").to_string();
        let handle = std::thread::spawn(move || {
            for _ in 0..requests {
                let (mut s, _) = listener.accept().unwrap();
                let line = read_request(&mut s);
                if line.starts_with("POST /refresh") {
                    count.fetch_add(1, Ordering::SeqCst);
                    respond(
                        &mut s,
                        "200 OK",
                        &format!(r#"{{"discharge_macaroon": "{fresh}"}}"#),
                    );
                } else if line.contains("/v2/snaps/info/") && count.load(Ordering::SeqCst) == 0 {
                    respond(
                        &mut s,
                        "401 Unauthorized",
                        r#"{"error-list": [{"code": "macaroon-needs-refresh", "message": "refresh"}]}"#,
                    );
                } else if line.contains("/v2/snaps/info/nope") {
                    respond(&mut s, "404 Not Found", "{}");
                } else if line.contains("/v2/snaps/info/") {
                    respond(&mut s, "200 OK", &map);
                } else {
                    let mut page: Value = serde_json::from_str(&releases).unwrap();
                    page["_links"] =
                        serde_json::json!({"next": {"href": "http://elsewhere.invalid/next"}});
                    respond(&mut s, "200 OK", &page.to_string());
                }
            }
        });
        let ep = Endpoints {
            api: base.clone(),
            dashboard: base.clone(),
            refresh: format!("{base}/refresh"),
        };
        (ep, refreshes, handle)
    }

    #[test]
    fn one_refresh_for_concurrent_requests() {
        macaroon::initialize().unwrap();
        // two 401s at most before the refresh, one refresh, two retries
        // (or fewer 401s when the refresh comes first): accept up to 5
        let (ep, refreshes, _server) = server(5);
        let auth = SharedAuth::new(&test_credential());
        let snaps = vec!["a".to_string(), "b".to_string()];
        let got = channel_maps(&ep, Some(&auth), &snaps, Some("StoreId")).unwrap();
        assert_eq!(refreshes.load(Ordering::SeqCst), 1);
        assert_eq!(got.len(), 2);
        assert!(got
            .iter()
            .all(|g| g.as_ref().is_some_and(|(_, m)| m.len() == 3)));
    }

    #[test]
    fn hidden_snaps_and_foreign_next_links() {
        macaroon::initialize().unwrap();
        let (ep, refreshes, server) = server(3);
        // pretend the refresh already happened for the info answers
        refreshes.store(1, Ordering::SeqCst);
        let got = channel_maps(&ep, None, &["nope".to_string()], None).unwrap();
        assert_eq!(got, vec![None]);
        let auth = SharedAuth::new(&test_credential());
        let answer = releases(&ep, Some(&auth), "ce-dummy").unwrap();
        assert!(answer.stopped.unwrap().contains("another host"));
        assert_eq!(answer.history.unwrap().revisions.len(), 4);
        // the third accept is never reached: nothing went elsewhere; unblock it
        let _ = std::net::TcpStream::connect(ep.api.trim_start_matches("http://"));
        server.join().unwrap();
    }

    #[test]
    fn no_credential_no_call() {
        let ep = Endpoints {
            api: "http://127.0.0.1:9".into(),
            dashboard: "http://127.0.0.1:9".into(),
            refresh: "http://127.0.0.1:9".into(),
        };
        let a = releases(&ep, None, "x").unwrap();
        assert_eq!(a.history, Err(HistoryRefusal::NoCredential));
    }
}
