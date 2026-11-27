//! Fetching assertions from the store's assertions service, and the chain
//! of prerequisites snapd needs before it accepts one.
//!
//! `GET /v2/assertions/<type>/<primary key values>` with the assertion media
//! type; errors are JSON `error-list`s (404 `not-found`, 400
//! `invalid-request`). Snap names are resolved to snap-ids, and revisions to
//! their SHA3-384, through `/v2/snaps/info` (released revisions) or the
//! refresh API's `download` action (any revision).

use super::query::{self, Auth, Reply, API};
use crate::error::StoreError;
use base64::prelude::*;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::HashSet;

const MEDIA_TYPE: &str = "application/x.ubuntu.assertion";

/// Canonical's account and root key, which snapd trusts built in: a chain
/// stops there.
pub const CANONICAL_ACCOUNT: &str = "canonical";
pub const CANONICAL_ROOT_KEY: &str =
    "-CvQKAwRQ5h3Ffn10FILJoEZUXOv6km9FwA80-Rcj-f-6jadQ89VRswHNiEB9Lxk";

/// An assertion type the store serves: its primary-key headers in order,
/// an optional trailing key left to the store's default, whether it forms
/// sequences (the last key is the sequence), and the `max-format` snapd asks
/// for. From snapd's asserts/asserts.go and store/store_asserts.go.
pub struct Kind {
    pub name: &'static str,
    pub keys: &'static [&'static str],
    pub optional: Option<&'static str>,
    pub sequence: bool,
    pub max_format: u32,
}

const fn kind(name: &'static str, keys: &'static [&'static str]) -> Kind {
    Kind {
        name,
        keys,
        optional: None,
        sequence: false,
        max_format: 0,
    }
}

pub const KINDS: &[Kind] = &[
    kind("account", &["account-id"]),
    Kind {
        max_format: 1,
        ..kind("account-key", &["public-key-sha3-384"])
    },
    Kind {
        sequence: true,
        ..kind("repair", &["brand-id", "repair-id"])
    },
    kind("model", &["series", "brand-id", "model"]),
    kind("serial", &["brand-id", "model", "serial"]),
    kind("base-declaration", &["series"]),
    Kind {
        max_format: 7,
        ..kind("snap-declaration", &["series", "snap-id"])
    },
    kind("snap-build", &["snap-sha3-384"]),
    Kind {
        optional: Some("provenance"),
        ..kind("snap-revision", &["snap-sha3-384"])
    },
    kind("snap-developer", &["snap-id", "publisher-id"]),
    Kind {
        max_format: 2,
        ..kind("system-user", &["brand-id", "email"])
    },
    kind(
        "validation",
        &[
            "series",
            "snap-id",
            "approved-snap-id",
            "approved-snap-revision",
        ],
    ),
    Kind {
        sequence: true,
        ..kind(
            "validation-set",
            &["series", "account-id", "name", "sequence"],
        )
    },
    kind("store", &["store"]),
    kind("preseed", &["series", "brand-id", "model", "system-label"]),
    Kind {
        optional: Some("provenance"),
        ..kind(
            "snap-resource-revision",
            &["snap-id", "resource-name", "resource-sha3-384"],
        )
    },
    Kind {
        optional: Some("provenance"),
        ..kind(
            "snap-resource-pair",
            &[
                "snap-id",
                "resource-name",
                "resource-revision",
                "snap-revision",
            ],
        )
    },
    kind("confdb-schema", &["account-id", "name"]),
    Kind {
        sequence: true,
        ..kind("cluster", &["cluster-id", "sequence"])
    },
    kind("request-message", &["account-id", "message-id"]),
    kind(
        "hardware-identity",
        &["issuer-id", "hardware-id-key-sha3-384"],
    ),
];

pub fn kind_named(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|k| k.name == name)
}

fn usage(kind: &Kind) -> String {
    let mut keys: Vec<String> = kind
        .keys
        .iter()
        .map(|k| match *k {
            "series" => "[series=16]".to_string(),
            "sequence" if kind.sequence => "[sequence=]".to_string(),
            k => format!("{k}="),
        })
        .collect();
    if let Some(o) = kind.optional {
        keys.push(format!("[{o}=]"));
    }
    format!("{} is fetched by: {}", kind.name, keys.join(" "))
}

/// What to fetch: a type and its primary-key values, or lookups to resolve
/// first (`snap-name=`, `snap-name=` + `snap-revision=` or `snap-channel=`).
#[derive(Debug, PartialEq, Clone)]
pub struct Request {
    pub kind: &'static str,
    /// (header, value) in the type's key order; values may be pending
    /// lookups, recorded in `snap_name` / `snap_revision` / `snap_channel`.
    pub headers: Vec<(String, String)>,
    pub snap_name: Option<String>,
    pub snap_revision: Option<u64>,
    pub snap_channel: Option<String>,
}

/// Parses `<type> <header>=<value>...` into a request, failing (usage) on
/// an unknown type, an unknown or repeated header, or a missing key.
pub fn parse(kind_name: &str, words: &[String]) -> Result<Request, String> {
    let kind = kind_named(kind_name).ok_or_else(|| {
        let names: Vec<&str> = KINDS.iter().map(|k| k.name).collect();
        format!(
            "unknown assertion type '{kind_name}' (known: {})",
            names.join(", ")
        )
    })?;
    let mut given: Vec<(String, String)> = Vec::new();
    for w in words {
        let Some((h, v)) = w.split_once('=') else {
            return Err(format!("'{w}' isn't header=value — {}", usage(kind)));
        };
        if v.is_empty() {
            return Err(format!("{h} is empty — {}", usage(kind)));
        }
        if given.iter().any(|(g, _)| g == h) {
            return Err(format!("{h} given twice"));
        }
        let allowed = kind.keys.contains(&h)
            || kind.optional == Some(h)
            || (h == "snap-name"
                && (kind.keys.contains(&"snap-id") || kind.keys.contains(&"snap-sha3-384")))
            || ((h == "snap-revision" || h == "snap-channel")
                && kind.keys.contains(&"snap-sha3-384"));
        if !allowed {
            return Err(format!("unknown header {h} — {}", usage(kind)));
        }
        given.push((h.to_string(), v.to_string()));
    }
    let get = |h: &str| given.iter().find(|(g, _)| g == h).map(|(_, v)| v.clone());
    let snap_name = get("snap-name");
    let snap_revision = match get("snap-revision") {
        Some(r) => Some(
            r.parse::<u64>()
                .map_err(|_| format!("snap-revision '{r}' isn't a revision number"))?,
        ),
        None => None,
    };
    let snap_channel = get("snap-channel");
    if snap_revision.is_some() && snap_name.is_none() {
        return Err("snap-revision= needs snap-name= too".into());
    }
    if snap_channel.is_some() && snap_name.is_none() {
        return Err("snap-channel= needs snap-name= too".into());
    }
    if snap_channel.is_some() && snap_revision.is_some() {
        return Err("give snap-revision= or snap-channel=, not both".into());
    }
    if let Some(c) = &snap_channel {
        Channel::parse(c)?;
    }
    let lookup = snap_revision.is_some() || snap_channel.is_some();
    let mut headers = Vec::new();
    for k in kind.keys {
        let value = match (*k, get(k)) {
            (_, Some(v)) => v,
            ("series", None) => "16".into(),
            ("snap-id", None) if snap_name.is_some() => String::new(),
            ("snap-sha3-384", None) if lookup => String::new(),
            ("sequence", None) if kind.sequence => String::new(),
            _ => return Err(format!("{k} is missing — {}", usage(kind))),
        };
        headers.push((k.to_string(), value));
    }
    if let Some(o) = kind.optional {
        if let Some(v) = get(o) {
            headers.push((o.to_string(), v));
        }
    }
    // a snap-name alone resolves snap-id, not a revision's hash
    if kind.keys.contains(&"snap-sha3-384") && !lookup && get("snap-sha3-384").is_none() {
        return Err(format!("snap-sha3-384 is missing — {}", usage(kind)));
    }
    Ok(Request {
        kind: kind.name,
        headers,
        snap_name,
        snap_revision,
        snap_channel,
    })
}

const RISKS: [&str; 4] = ["stable", "candidate", "beta", "edge"];

/// A channel as given: `<risk>`, `<risk>/<branch>`, `<track>/<risk>` or
/// `<track>/<risk>/<branch>`; no track means the snap's default track.
#[derive(Debug, PartialEq)]
pub struct Channel {
    pub track: Option<String>,
    pub risk: String,
    pub branch: Option<String>,
}

impl Channel {
    pub fn parse(c: &str) -> Result<Channel, String> {
        let parts: Vec<&str> = c.split('/').collect();
        let is_risk = |p: &str| RISKS.contains(&p);
        let ch = |t: Option<&str>, r: &str, b: Option<&str>| Channel {
            track: t.map(str::to_owned),
            risk: r.to_owned(),
            branch: b.map(str::to_owned),
        };
        match parts.as_slice() {
            [r] if is_risk(r) => Ok(ch(None, r, None)),
            [t] if !t.is_empty() => Err(format!(
                "'{c}' isn't a channel — a track alone is written {t}/stable"
            )),
            [r, b] if is_risk(r) && !b.is_empty() => Ok(ch(None, r, Some(b))),
            [t, r] if !t.is_empty() && is_risk(r) => Ok(ch(Some(t), r, None)),
            [t, r, b] if !t.is_empty() && is_risk(r) && !b.is_empty() => {
                Ok(ch(Some(t), r, Some(b)))
            }
            _ => Err(format!(
                "'{c}' isn't a channel (<risk>, <track>/<risk> or <track>/<risk>/<branch>, risk one of {})",
                RISKS.join(", ")
            )),
        }
    }
}

/// What a channel resolved to: the channel released into, its revision and
/// the revision's SHA3-384.
#[derive(Debug, PartialEq)]
pub struct Release {
    pub channel: String,
    pub revision: u64,
    pub sha3_384: String,
}

/// The release snapd would install from `channel` for `arch`, from an info
/// reply's channel map: a risk with nothing released follows the next more
/// stable risk in its track (a branch is taken only as given). None when
/// nothing is released there for that architecture.
pub fn channel_release(info: &Value, channel: &Channel, arch: Option<&str>) -> Option<Release> {
    let map = info["channel-map"].as_array()?;
    let track = channel
        .track
        .clone()
        .or_else(|| info["default-track"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "latest".into());
    // the store names latest's channels without the track ("stable",
    // "edge/fix"): match the track and risk fields, the branch from the name
    let branch_of = |c: &Value| {
        let (t, r) = (c["track"].as_str()?, c["risk"].as_str()?);
        let name = c["name"].as_str()?;
        let rest = name
            .strip_prefix(&format!("{t}/{r}"))
            .or_else(|| name.strip_prefix(r))?;
        Some(rest.strip_prefix('/').unwrap_or(rest).to_owned())
    };
    let find = |risk: &str, branch: Option<&str>| {
        map.iter().find(|e| {
            let c = &e["channel"];
            c["track"].as_str() == Some(track.as_str())
                && c["risk"].as_str() == Some(risk)
                && branch_of(c).as_deref() == Some(branch.unwrap_or(""))
                && arch.is_none_or(|a| c["architecture"].as_str() == Some(a))
        })
    };
    let entry = match &channel.branch {
        Some(b) => find(&channel.risk, Some(b)),
        None => {
            let from = RISKS.iter().position(|r| *r == channel.risk)?;
            RISKS[..=from].iter().rev().find_map(|r| find(r, None))
        }
    }?;
    let c = &entry["channel"];
    let mut name = format!("{}/{}", c["track"].as_str()?, c["risk"].as_str()?);
    if let Some(b) = branch_of(c).filter(|b| !b.is_empty()) {
        name = format!("{name}/{b}");
    }
    Some(Release {
        channel: name,
        revision: entry["revision"].as_u64()?,
        sha3_384: entry["download"]["sha3-384"].as_str()?.to_owned(),
    })
}

/// A SHA3-384 in the store's key form: base64url without padding. Hex (96
/// digits) is converted; anything else is passed as given.
pub fn sha3_key(value: &str) -> String {
    if value.len() == 96 && value.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes: Vec<u8> = (0..48)
            .map(|i| u8::from_str_radix(&value[2 * i..2 * i + 2], 16).unwrap_or(0))
            .collect();
        BASE64_URL_SAFE_NO_PAD.encode(bytes)
    } else {
        value.trim_end_matches('=').to_string()
    }
}

/// The path (after `/v2/assertions/`) and query for a resolved request.
pub fn path(req: &Request) -> String {
    let kind = kind_named(req.kind).expect("parsed kinds are known");
    let mut parts = Vec::new();
    let mut query = Vec::new();
    for (h, v) in &req.headers {
        if h == "sequence" && v.is_empty() {
            query.push("sequence=latest".to_string());
            continue;
        }
        let v = if h.ends_with("sha3-384") {
            sha3_key(v)
        } else {
            v.clone()
        };
        parts.push(v);
    }
    if kind.max_format > 0 {
        query.push(format!("max-format={}", kind.max_format));
    }
    let mut p = format!("{}/{}", kind.name, parts.join("/"));
    if !query.is_empty() {
        p = format!("{p}?{}", query.join("&"));
    }
    p
}

/// Where the fetches go and how: the client, an optional credential, the
/// store ID and the architecture for revision lookups.
pub struct Fetcher<'a> {
    client: Client,
    auth: Option<&'a mut Auth>,
    store: Option<String>,
    arch: Option<String>,
}

/// Something that can fetch an assertion by type and key path: the store,
/// or a test double.
pub trait Source {
    fn fetch(&mut self, path: &str) -> Result<String, StoreError>;
}

impl<'a> Fetcher<'a> {
    pub fn new(
        auth: Option<&'a mut Auth>,
        store: Option<String>,
        arch: Option<String>,
    ) -> Result<Self, StoreError> {
        Ok(Fetcher {
            client: query::client()?,
            auth,
            store,
            arch,
        })
    }

    fn send(
        &mut self,
        build: impl Fn(&Client) -> reqwest::blocking::RequestBuilder,
    ) -> Result<Reply, StoreError> {
        let c = &self.client;
        let store = self.store.clone();
        query::send(c, self.auth.as_deref_mut(), || {
            let mut rq = build(c);
            if let Some(id) = &store {
                rq = rq.header("Snap-Device-Store", id);
            }
            rq
        })
    }

    /// Resolves a request's lookups (snap-id, a revision's SHA3-384).
    pub fn resolve(&mut self, mut req: Request) -> Result<Request, StoreError> {
        let Some(name) = req.snap_name.clone() else {
            return Ok(req);
        };
        // send() adds Snap-Device-Store itself; a second copy hides the snap
        let reply = self.send(|c| query::info_request(c, &name, None))?;
        let info = match reply.status {
            StatusCode::NOT_FOUND => {
                return Err(StoreError::NotFound(format!(
                    "the store doesn't show {name} to this credential"
                )))
            }
            s if s.is_success() => reply.json(),
            _ => return Err(query::rejected_or(&reply)),
        };
        let snap_id = query::text(&info, "snap-id");
        for (h, v) in req.headers.iter_mut() {
            if h == "snap-id" && v.is_empty() {
                *v = snap_id.clone();
            }
        }
        if let Some(rev) = req.snap_revision {
            let hash = match revision_hash(&info, rev, self.arch.as_deref()) {
                Some(h) => h,
                None => self.refresh_hash(&name, rev)?,
            };
            for (h, v) in req.headers.iter_mut() {
                if h == "snap-sha3-384" && v.is_empty() {
                    *v = hash.clone();
                }
            }
        }
        if let Some(given) = req.snap_channel.clone() {
            let channel = Channel::parse(&given).map_err(StoreError::Store)?;
            let arch = self.arch.clone();
            let rel = channel_release(&info, &channel, arch.as_deref()).ok_or_else(|| {
                StoreError::NotFound(format!(
                    "{name} has nothing released in {given} for {}",
                    arch.as_deref().unwrap_or("any architecture")
                ))
            })?;
            eprintln!("channel\t{given}\t{}\t{}", rel.channel, rel.revision);
            for (h, v) in req.headers.iter_mut() {
                if h == "snap-sha3-384" && v.is_empty() {
                    *v = rel.sha3_384.clone();
                }
            }
        }
        req.snap_name = None;
        req.snap_revision = None;
        req.snap_channel = None;
        Ok(req)
    }

    /// A revision outside the channel map: the refresh API's download
    /// action names it (as `snap download --revision` asks).
    fn refresh_hash(&mut self, name: &str, rev: u64) -> Result<String, StoreError> {
        let arch = self.arch.clone().unwrap_or_else(|| "amd64".into());
        let body = json!({
            "context": [],
            "actions": [{"action": "download", "instance-key": "download-1", "name": name, "revision": rev}],
            "fields": ["download", "revision"],
        });
        let reply = self.send(|c| {
            c.post(format!("{API}/v2/snaps/refresh"))
                .header("Snap-Device-Series", "16")
                .header("Snap-Device-Architecture", &arch)
                .header("Accept", "application/json")
                .json(&body)
        })?;
        if !reply.status.is_success() {
            return Err(query::rejected_or(&reply));
        }
        refresh_result_hash(&reply.json()).ok_or_else(|| {
            StoreError::NotFound(format!(
                "the store has no revision {rev} of {name} for {arch}, or doesn't show it to this credential"
            ))
        })
    }
}

impl Source for Fetcher<'_> {
    fn fetch(&mut self, path: &str) -> Result<String, StoreError> {
        let reply = self.send(|c| {
            c.get(format!("{API}/v2/assertions/{path}"))
                .header("Accept", MEDIA_TYPE)
        })?;
        assertion_reply(&reply, path)
    }
}

/// The assertion in a reply, or the error its status and body mean.
pub fn assertion_reply(reply: &Reply, path: &str) -> Result<String, StoreError> {
    match reply.status {
        s if s.is_success() => Ok(reply.body.clone()),
        StatusCode::NOT_FOUND => {
            let key = path.split('?').next().unwrap_or(path).replacen('/', " ", 1);
            Err(StoreError::NotFound(format!(
                "the store has no {key}, or doesn't show it to this credential"
            )))
        }
        StatusCode::UNAUTHORIZED => Err(StoreError::Rejected),
        _ => Err(query::rejected_or(reply)),
    }
}

/// A released revision's SHA3-384 (hex) from a v2 info channel map: for
/// `arch` when given, else any architecture.
pub fn revision_hash(info: &Value, rev: u64, arch: Option<&str>) -> Option<String> {
    let map = info["channel-map"].as_array()?;
    let pick = |want_arch: bool| {
        map.iter().find(|e| {
            e["revision"].as_u64() == Some(rev)
                && (!want_arch
                    || arch.is_none_or(|a| e["channel"]["architecture"].as_str() == Some(a)))
        })
    };
    pick(true)
        .or_else(|| pick(false))
        .and_then(|e| e["download"]["sha3-384"].as_str())
        .map(str::to_owned)
}

/// The SHA3-384 in a refresh answer's first result, or None (e.g.
/// `revision-not-found`).
pub fn refresh_result_hash(answer: &Value) -> Option<String> {
    let r = answer["results"].as_array()?.first()?;
    if r["result"].as_str() != Some("download") {
        return None;
    }
    r["snap"]["download"]["sha3-384"]
        .as_str()
        .map(str::to_owned)
}

/// An assertion's top-level headers (`name: value` lines before the body),
/// and the `developer-id`s in a `developers` list. Enough to follow a chain.
pub fn headers(assertion: &str) -> (Vec<(String, String)>, Vec<String>) {
    let head = assertion.split("\n\n").next().unwrap_or("");
    let mut top = Vec::new();
    let mut developers = Vec::new();
    for line in head.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("developer-id:") {
            if line.starts_with(' ') {
                developers.push(rest.trim().to_string());
                continue;
            }
        }
        if line.starts_with(' ') {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            top.push((k.to_string(), v.trim().to_string()));
        }
    }
    (top, developers)
}

fn header<'a>(h: &'a [(String, String)], k: &str) -> Option<&'a str> {
    h.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
}

/// The paths of what snapd needs before it accepts `assertion`: its signing
/// key, and the assertions it refers to.
pub fn prerequisites(assertion: &str) -> Vec<String> {
    let (h, developers) = headers(assertion);
    let mut out = Vec::new();
    let account = |id: &str| format!("account/{id}");
    if let Some(key) = header(&h, "sign-key-sha3-384") {
        out.push(format!("account-key/{key}?max-format=1"));
    }
    match header(&h, "type").unwrap_or("") {
        "account-key" => out.extend(header(&h, "account-id").map(account)),
        "snap-declaration" => out.extend(header(&h, "publisher-id").map(account)),
        "snap-revision" => {
            if let Some(id) = header(&h, "snap-id") {
                out.push(format!("snap-declaration/16/{id}?max-format=7"));
            }
            out.extend(header(&h, "developer-id").map(account));
        }
        "snap-developer" => {
            if let Some(id) = header(&h, "snap-id") {
                out.push(format!("snap-declaration/16/{id}?max-format=7"));
            }
            out.extend(header(&h, "publisher-id").map(account));
            out.extend(developers.iter().map(|d| account(d)));
        }
        "store" => out.extend(header(&h, "operator-id").map(account)),
        _ => {}
    }
    out.retain(|p| {
        p != &format!("account/{CANONICAL_ACCOUNT}")
            && !p.starts_with(&format!("account-key/{CANONICAL_ROOT_KEY}"))
    });
    out
}

/// A key identifying an assertion (its path without the query).
fn key_of(path: &str) -> String {
    path.split('?').next().unwrap_or(path).to_string()
}

/// The assertion at `path` and, with `chain`, everything it needs, each
/// once, every assertion after the ones it needs. Nothing is returned
/// until all are fetched.
pub fn fetch(source: &mut dyn Source, path: &str, chain: bool) -> Result<Vec<String>, StoreError> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    visit(source, path, chain, &mut seen, &mut out)?;
    Ok(out)
}

fn visit(
    source: &mut dyn Source,
    path: &str,
    chain: bool,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) -> Result<(), StoreError> {
    if !seen.insert(key_of(path)) {
        return Ok(());
    }
    let assertion = source.fetch(path)?;
    if chain {
        for p in prerequisites(&assertion) {
            visit(source, &p, chain, seen, out)?;
        }
    }
    out.push(assertion);
    Ok(())
}

/// The stream as `snap known` prints it: each assertion as served, with one
/// blank line between them.
pub fn stream(assertions: &[String]) -> String {
    assertions
        .iter()
        .map(|a| a.trim_end_matches('\n').to_string())
        .collect::<Vec<_>>()
        .join("\n\n")
        + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn fixture(name: &str) -> String {
        let p = format!(
            "{}/src/testdata/assertions/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(p).unwrap()
    }

    fn words(w: &[&str]) -> Vec<String> {
        w.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn paths_from_headers() {
        let r = parse("account", &words(&["account-id=canonical"])).unwrap();
        assert_eq!(path(&r), "account/canonical");
        let r = parse(
            "model",
            &words(&["brand-id=canonical", "model=ubuntu-core-24-amd64"]),
        )
        .unwrap();
        assert_eq!(path(&r), "model/16/canonical/ubuntu-core-24-amd64");
        let r = parse(
            "snap-declaration",
            &words(&["snap-id=buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ"]),
        )
        .unwrap();
        assert_eq!(
            path(&r),
            "snap-declaration/16/buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ?max-format=7"
        );
        let r = parse(
            "validation-set",
            &words(&["account-id=acme", "name=base-set"]),
        )
        .unwrap();
        assert_eq!(path(&r), "validation-set/16/acme/base-set?sequence=latest");
        let r = parse(
            "validation-set",
            &words(&["account-id=acme", "name=base-set", "sequence=3"]),
        )
        .unwrap();
        assert_eq!(path(&r), "validation-set/16/acme/base-set/3");
        let r = parse(
            "serial",
            &words(&["brand-id=acme", "model=gw", "serial=1234"]),
        )
        .unwrap();
        assert_eq!(path(&r), "serial/acme/gw/1234");
    }

    #[test]
    fn hex_hash_becomes_base64url() {
        let hex = fixture("hello-world-29.sha3-384");
        let r = parse(
            "snap-revision",
            &words(&[&format!("snap-sha3-384={}", hex.trim())]),
        )
        .unwrap();
        let p = path(&r);
        // the fixture was fetched by its base64url key
        assert!(fixture("snap-revision-hello-world-29").contains(&format!(
            "snap-sha3-384: {}",
            p.trim_start_matches("snap-revision/")
        )));
    }

    #[test]
    fn errors_name_the_key_shape() {
        let e = parse("model", &words(&["brand-id=canonical"])).unwrap_err();
        assert!(
            e.contains("model is missing") && e.contains("brand-id= model="),
            "{e}"
        );
        assert!(parse("nope", &[])
            .unwrap_err()
            .contains("known: account, account-key"));
        assert!(parse("account", &words(&["bogus=1"]))
            .unwrap_err()
            .contains("unknown header bogus"));
        assert!(parse("account", &words(&["canonical"]))
            .unwrap_err()
            .contains("isn't header=value"));
        assert!(parse("snap-revision", &words(&["snap-name=hello-world"]))
            .unwrap_err()
            .contains("snap-sha3-384 is missing"));
        assert!(parse(
            "snap-revision",
            &words(&["snap-name=x", "snap-revision=abc"])
        )
        .unwrap_err()
        .contains("isn't a revision"));
    }

    #[test]
    fn lookups_pending() {
        let r = parse(
            "snap-revision",
            &words(&["snap-name=hello-world", "snap-revision=29"]),
        )
        .unwrap();
        assert_eq!(r.snap_name.as_deref(), Some("hello-world"));
        assert_eq!(r.snap_revision, Some(29));
        let r = parse("snap-declaration", &words(&["snap-name=hello-world"])).unwrap();
        assert_eq!(
            r.headers,
            [
                ("series".into(), "16".into()),
                ("snap-id".into(), String::new())
            ]
        );
    }

    #[test]
    fn revision_hashes() {
        let info = json!({"channel-map": [
            {"revision": 29, "channel": {"architecture": "amd64"}, "download": {"sha3-384": "aa"}},
            {"revision": 30, "channel": {"architecture": "arm64"}, "download": {"sha3-384": "bb"}},
        ]});
        assert_eq!(
            revision_hash(&info, 29, Some("amd64")).as_deref(),
            Some("aa")
        );
        assert_eq!(
            revision_hash(&info, 30, Some("amd64")).as_deref(),
            Some("bb")
        );
        assert_eq!(revision_hash(&info, 31, None), None);
        let ok: Value = serde_json::from_str(&fixture("refresh-download.json")).unwrap();
        assert!(refresh_result_hash(&ok).unwrap().starts_with("eed62063c0"));
        let missing: Value = serde_json::from_str(&fixture("refresh-missing.json")).unwrap();
        assert_eq!(refresh_result_hash(&missing), None);
    }

    #[test]
    fn channels_parse() {
        let c = |t: Option<&str>, r: &str, b: Option<&str>| Channel {
            track: t.map(str::to_owned),
            risk: r.into(),
            branch: b.map(str::to_owned),
        };
        assert_eq!(Channel::parse("stable"), Ok(c(None, "stable", None)));
        assert_eq!(
            Channel::parse("edge/fix-1"),
            Ok(c(None, "edge", Some("fix-1")))
        );
        assert_eq!(Channel::parse("2.0/beta"), Ok(c(Some("2.0"), "beta", None)));
        assert_eq!(
            Channel::parse("latest/edge/fix-1"),
            Ok(c(Some("latest"), "edge", Some("fix-1")))
        );
        assert!(Channel::parse("22").unwrap_err().contains("22/stable"));
        assert!(Channel::parse("2.0/nightly").is_err());
        assert!(Channel::parse("a/stable/b/c").is_err());
        assert!(Channel::parse("").is_err());
    }

    #[test]
    fn channel_releases() {
        // as the store gives them: latest's channels named without the track
        let e = |name: &str, arch: &str, rev: u64| {
            let parts: Vec<&str> = name.split('/').collect();
            let (track, risk) = match parts.as_slice() {
                [t, r, ..] if !super::RISKS.contains(t) => (*t, *r),
                [r, ..] => ("latest", *r),
                [] => unreachable!(),
            };
            json!({"channel": {"name": name, "track": track, "risk": risk, "architecture": arch},
                   "revision": rev, "download": {"sha3-384": format!("h{rev}")}})
        };
        let info = json!({"default-track": "6", "channel-map": [
            e("6/stable", "amd64", 10), e("6/beta", "amd64", 12),
            e("6/stable", "arm64", 11), e("stable", "amd64", 20),
            e("edge/fix-1", "amd64", 25),
        ]});
        let rel = |c: &str, a: Option<&str>| {
            channel_release(&info, &Channel::parse(c).unwrap(), a).map(|r| (r.channel, r.revision))
        };
        // a risk alone takes the default track
        assert_eq!(rel("stable", Some("amd64")), Some(("6/stable".into(), 10)));
        assert_eq!(rel("stable", Some("arm64")), Some(("6/stable".into(), 11)));
        // closed edge follows beta; candidate follows stable
        assert_eq!(rel("edge", Some("amd64")), Some(("6/beta".into(), 12)));
        assert_eq!(
            rel("6/candidate", Some("amd64")),
            Some(("6/stable".into(), 10))
        );
        assert_eq!(
            rel("latest/stable", Some("amd64")),
            Some(("latest/stable".into(), 20))
        );
        // branches only as given; nothing for an architecture
        assert_eq!(
            rel("latest/edge/fix-1", Some("amd64")),
            Some(("latest/edge/fix-1".into(), 25))
        );
        assert_eq!(rel("latest/edge/nope", Some("amd64")), None);
        assert_eq!(rel("latest/stable", Some("riscv64")), None);
        assert_eq!(rel("stable", Some("riscv64")), None);
        let r = channel_release(&info, &Channel::parse("stable").unwrap(), Some("amd64")).unwrap();
        assert_eq!(r.sha3_384, "h10");
        // no default track: latest
        let plain = json!({"channel-map": [e("stable", "amd64", 3)]});
        assert_eq!(
            channel_release(&plain, &Channel::parse("stable").unwrap(), Some("amd64"))
                .map(|r| r.revision),
            Some(3)
        );
    }

    #[test]
    fn channel_requests() {
        let r = parse(
            "snap-revision",
            &words(&["snap-name=hello-world", "snap-channel=stable"]),
        )
        .unwrap();
        assert_eq!(r.snap_channel.as_deref(), Some("stable"));
        assert!(parse("snap-revision", &words(&["snap-channel=stable"])).is_err());
        assert!(parse(
            "snap-revision",
            &words(&["snap-name=x", "snap-channel=stable", "snap-revision=3"])
        )
        .is_err());
        assert!(parse("snap-revision", &words(&["snap-name=x", "snap-channel=22"])).is_err());
        assert!(parse(
            "snap-declaration",
            &words(&["snap-name=x", "snap-channel=stable"])
        )
        .is_err());
    }

    #[test]
    fn error_replies() {
        let r = |code: u16, name: &str| Reply {
            status: StatusCode::from_u16(code).unwrap(),
            body: fixture(name),
            wants_refresh: false,
        };
        let e = assertion_reply(&r(404, "error-not-found.json"), "account/no-such").unwrap_err();
        assert_eq!(e.exit_code(), 4);
        assert!(e.to_string().contains("no account no-such"), "{e}");
        let e = assertion_reply(&r(400, "error-invalid-request.json"), "model/16/canonical")
            .unwrap_err();
        assert_eq!(e.exit_code(), 3);
    }

    /// Serves the recorded fixtures by path, counting fetches.
    struct Fixtures {
        by_key: HashMap<String, String>,
        fetched: Vec<String>,
    }

    impl Source for Fixtures {
        fn fetch(&mut self, path: &str) -> Result<String, StoreError> {
            self.fetched.push(key_of(path));
            self.by_key
                .get(&key_of(path))
                .cloned()
                .ok_or_else(|| StoreError::NotFound(format!("no {path}")))
        }
    }

    fn fixtures() -> Fixtures {
        let rev = fixture("snap-revision-hello-world-29");
        let (h, _) = headers(&rev);
        let sha = header(&h, "snap-sha3-384").unwrap().to_string();
        let mut by_key = HashMap::new();
        by_key.insert(format!("snap-revision/{sha}"), rev);
        by_key.insert(
            "snap-declaration/16/buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ".into(),
            fixture("snap-declaration-hello-world"),
        );
        by_key.insert(
            "account-key/BWDEoaqyr25nF5SNCvEv2v7QnM9QsfCc0PBMYD_i2NGSQ32EF2d4D0hqUel3m8ul".into(),
            fixture("account-key-store"),
        );
        by_key.insert(
            format!("account-key/{CANONICAL_ROOT_KEY}"),
            fixture("account-key-root"),
        );
        by_key.insert("account/canonical".into(), fixture("account-canonical"));
        Fixtures {
            by_key,
            fetched: Vec::new(),
        }
    }

    #[test]
    fn chain_for_a_revision() {
        let mut f = fixtures();
        let rev = fixture("snap-revision-hello-world-29");
        let (h, _) = headers(&rev);
        let path = format!("snap-revision/{}", header(&h, "snap-sha3-384").unwrap());
        let got = fetch(&mut f, &path, true).unwrap();
        let types: Vec<String> = got
            .iter()
            .map(|a| header(&headers(a).0, "type").unwrap().to_string())
            .collect();
        assert_eq!(types, ["account-key", "snap-declaration", "snap-revision"]);
        // the store key once, though both the revision and the declaration need it
        assert_eq!(
            f.fetched
                .iter()
                .filter(|p| p.starts_with("account-key/"))
                .count(),
            1
        );
        assert!(!f.fetched.contains(&"account/canonical".to_string()));
        let s = stream(&got);
        assert_eq!(
            s.matches("\ntype: ").count() + usize::from(s.starts_with("type: ")),
            3
        );
        assert!(s.ends_with('\n') && !s.ends_with("\n\n"));
    }

    #[test]
    fn missing_prerequisite_fails() {
        let mut f = fixtures();
        f.by_key
            .remove("snap-declaration/16/buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ");
        let rev = fixture("snap-revision-hello-world-29");
        let path = format!(
            "snap-revision/{}",
            header(&headers(&rev).0, "snap-sha3-384").unwrap()
        );
        assert!(fetch(&mut f, &path, true).is_err());
    }

    #[test]
    fn no_chain_is_one_fetch() {
        let mut f = fixtures();
        let got = fetch(&mut f, "account/canonical", false).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(f.fetched.len(), 1);
    }
}
