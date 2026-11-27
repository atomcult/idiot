//! `draft`: unsigned model, system-user and validation-set assertions as
//! the JSON `snap sign` takes (every scalar a string), for `idiot
//! assertion`.
//!
//! `draft write <type>` reads tab-separated records on stdin and prints the
//! JSON draft:
//!   h <header> <value>   set a scalar header
//!   l <header> <value>   append to a list header
//!   s <name>             start a `snaps` entry
//!   v <name>             start a `validation-sets` entry (a model's)
//!   f <field> <value>    set a field of the current entry
//!   x <path> <json>      set a value idiot doesn't model, as JSON:
//!                        `<header>`, `snaps[<name>].<field>` or
//!                        `validation-sets[<name>].<field>`
//!   t <type>             the draft's type (ignored when it matches)
//! Headers come out in a fixed order (`type`, `authority-id`, the primary
//! keys, the rest alphabetically, `snaps` last), indented by two spaces.
//!
//! `draft read <file|->` prints those records for a JSON draft, `t` first,
//! so that `read | write` gives the same object back. A signed assertion
//! (or a stream holding exactly one model, system-user or validation-set,
//! as `--chain` and `snap known` give) is read as the draft it was signed
//! from: every header as a string, without `sign-key-sha3-384`,
//! `body-length` and the signature, which isn't checked. After `t` comes
//!   o <signed|draft> <type> <keys> <revision>
//! (`keys`: the primary keys but `series`, joined by `/`; `revision` 0
//! when absent), which `draft write` ignores. Other types are refused.
//!
//! `draft check <file|->` prints `error\t<where>\t<message>` and
//! `warning\t<where>\t<message>` lines for what snapd would refuse when
//! signing (asserts/model.go, system_user.go, validation_set.go at snapd
//! 92cc6565e7), `<where>` a header or `snaps[<name>].<field>`.
//!
//! Exit status: 0 ok, 1 an invalid draft (or `check` found an error), 2
//! usage or unreadable input. No network code; it is only ever given a
//! password's hash.

use crate::assertion;
use serde_json::{Map, Value};
use std::io::{Read, Write};

const USAGE: &str = "usage: idiot-helper draft write <model|system-user|validation-set>
       idiot-helper draft read <file|->
       idiot-helper draft check <file|->";

const TYPES: &[&str] = &["model", "system-user", "validation-set"];

pub fn run(args: &[String]) -> u8 {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let res = match a.as_slice() {
        ["write", ty] if TYPES.contains(ty) => read_input("-").and_then(|b| {
            let text = String::from_utf8(b).map_err(|_| (1, "records aren't UTF-8".into()))?;
            write(ty, &text)
        }),
        ["read", path] => read_input(path).and_then(|b| read(&b)),
        ["check", path] => {
            return match read_input(path) {
                Ok(b) => check_cmd(&b),
                Err((code, msg)) => {
                    eprintln!("idiot-helper draft: {msg}");
                    code
                }
            }
        }
        _ => {
            eprintln!("{USAGE}");
            return 2;
        }
    };
    match res {
        Ok(out) => {
            if std::io::stdout().lock().write_all(out.as_bytes()).is_err() {
                return 2;
            }
            0
        }
        Err((code, msg)) => {
            eprintln!("idiot-helper draft: {msg}");
            code
        }
    }
}

type Res<T> = Result<T, (u8, String)>;

fn read_input(path: &str) -> Res<Vec<u8>> {
    if path == "-" {
        let mut b = Vec::new();
        std::io::stdin()
            .read_to_end(&mut b)
            .map_err(|e| (2, format!("stdin: {e}")))?;
        Ok(b)
    } else {
        std::fs::read(path).map_err(|e| (2, format!("{path}: {e}")))
    }
}

fn primary_keys(ty: &str) -> &'static [&'static str] {
    match ty {
        "model" => &["series", "brand-id", "model"],
        "system-user" => &["brand-id", "email"],
        _ => &["series", "account-id", "name", "sequence"],
    }
}

/// Field order inside a `snaps` or `validation-sets` entry; others follow
/// alphabetically.
const ENTRY_ORDER: &[&str] = &[
    "account-id",
    "name",
    "type",
    "default-channel",
    "id",
    "presence",
    "revision",
    "sequence",
    "mode",
    "modes",
    "components",
];

// ---- an ordered JSON value, printed by hand -----------------------------

#[derive(Debug, Clone)]
enum Node {
    Str(String),
    List(Vec<Node>),
    Obj(Vec<(String, Node)>),
    Raw(Value),
}

fn quote(s: &str) -> String {
    Value::String(s.into()).to_string()
}

fn pretty(n: &Node, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent + 1);
    let end = "  ".repeat(indent);
    match n {
        Node::Str(s) => out.push_str(&quote(s)),
        Node::Raw(v) => {
            let s = serde_json::to_string_pretty(v).unwrap_or_default();
            out.push_str(&s.replace('\n', &format!("\n{end}")));
        }
        Node::List(items) if items.is_empty() => out.push_str("[]"),
        Node::List(items) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                out.push_str(&pad);
                pretty(item, indent + 1, out);
                out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
            }
            out.push_str(&end);
            out.push(']');
        }
        Node::Obj(members) if members.is_empty() => out.push_str("{}"),
        Node::Obj(members) => {
            out.push_str("{\n");
            for (i, (k, v)) in members.iter().enumerate() {
                out.push_str(&format!("{pad}{}: ", quote(k)));
                pretty(v, indent + 1, out);
                out.push_str(if i + 1 < members.len() { ",\n" } else { "\n" });
            }
            out.push_str(&end);
            out.push('}');
        }
    }
}

fn set(members: &mut Vec<(String, Node)>, k: &str, v: Node) {
    match members.iter_mut().find(|(n, _)| n == k) {
        Some(m) => m.1 = v,
        None => members.push((k.into(), v)),
    }
}

fn entry_rank(k: &str) -> (usize, String) {
    (
        ENTRY_ORDER
            .iter()
            .position(|o| *o == k)
            .unwrap_or(ENTRY_ORDER.len()),
        k.to_string(),
    )
}

// ---- write ---------------------------------------------------------------

/// Which list an entry is in, and its index there.
type Current = Option<(&'static str, usize)>;

fn entry_name(e: &Node) -> Option<&str> {
    match e {
        Node::Obj(m) => m
            .iter()
            .find(|(k, _)| k == "name")
            .and_then(|(_, v)| match v {
                Node::Str(s) => Some(s.as_str()),
                _ => None,
            }),
        _ => None,
    }
}

fn entries<'a>(headers: &'a mut Vec<(String, Node)>, list: &str) -> &'a mut Vec<Node> {
    if !headers.iter().any(|(k, _)| k == list) {
        headers.push((list.into(), Node::List(Vec::new())));
    }
    match headers.iter_mut().find(|(k, _)| k == list) {
        Some((_, Node::List(l))) => l,
        Some(slot) => {
            slot.1 = Node::List(Vec::new());
            match &mut slot.1 {
                Node::List(l) => l,
                _ => unreachable!(),
            }
        }
        None => unreachable!(),
    }
}

pub fn write(ty: &str, records: &str) -> Res<String> {
    let mut headers: Vec<(String, Node)> = Vec::new();
    let mut current: Current = None;
    for (n, line) in records.lines().enumerate() {
        let bad = |m: &str| (1, format!("record {}: {m}", n + 1));
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let tag = parts.next().unwrap_or("");
        if tag == "o" {
            continue;
        }
        let a = parts.next();
        let b = parts.next();
        if b.is_some_and(|b| b.contains('\t')) {
            return Err(bad("a value can't hold a tab"));
        }
        match (tag, a, b) {
            ("t", Some(t), None) if t == ty => {}
            ("t", Some(t), None) => return Err(bad(&format!("the draft is a {t}, not a {ty}"))),
            ("h", Some(h), Some(v)) if !h.is_empty() => {
                if h == "type" {
                    if v != ty {
                        return Err(bad(&format!("type {v} in a {ty} draft")));
                    }
                    continue;
                }
                set(&mut headers, h, Node::Str(v.into()));
            }
            ("l", Some(h), Some(v)) if !h.is_empty() => {
                match headers.iter_mut().find(|(k, _)| k == h) {
                    Some((_, Node::List(l))) => l.push(Node::Str(v.into())),
                    Some(slot) => slot.1 = Node::List(vec![Node::Str(v.into())]),
                    None => headers.push((h.into(), Node::List(vec![Node::Str(v.into())]))),
                }
            }
            ("s" | "v", Some(name), None) if !name.is_empty() => {
                let list = if tag == "s" {
                    "snaps"
                } else {
                    "validation-sets"
                };
                let l = entries(&mut headers, list);
                l.push(Node::Obj(vec![("name".into(), Node::Str(name.into()))]));
                current = Some((list, l.len() - 1));
            }
            ("f", Some(field), Some(v)) if !field.is_empty() => {
                let Some((list, i)) = current else {
                    return Err(bad("a field before any s or v record"));
                };
                if let Node::Obj(m) = &mut entries(&mut headers, list)[i] {
                    set(m, field, Node::Str(v.into()));
                }
            }
            ("x", Some(path), Some(json)) => {
                let v: Value =
                    serde_json::from_str(json).map_err(|e| bad(&format!("{path}: {e}")))?;
                set_path(&mut headers, path, v).map_err(|m| bad(&m))?;
            }
            _ => return Err(bad(&format!("not a record: {line}"))),
        }
    }
    Ok(render(ty, headers))
}

fn set_path(headers: &mut Vec<(String, Node)>, path: &str, v: Value) -> Result<(), String> {
    for list in ["snaps", "validation-sets"] {
        let Some(rest) = path.strip_prefix(&format!("{list}[")) else {
            continue;
        };
        let (name, field) = rest
            .split_once("].")
            .ok_or_else(|| format!("bad path {path}"))?;
        let l = entries(headers, list);
        let e = l
            .iter_mut()
            .find(|e| entry_name(e) == Some(name))
            .ok_or_else(|| format!("{path}: no entry {name}"))?;
        if let Node::Obj(m) = e {
            set(m, field, Node::Raw(v));
        }
        return Ok(());
    }
    if path.is_empty() || path == "type" {
        return Err(format!("bad path {path}"));
    }
    set(headers, path, Node::Raw(v));
    Ok(())
}

fn render(ty: &str, mut headers: Vec<(String, Node)>) -> String {
    let keys = primary_keys(ty);
    let rank = |k: &str| -> (usize, String) {
        if k == "authority-id" {
            (0, String::new())
        } else if let Some(i) = keys.iter().position(|p| *p == k) {
            (1 + i, String::new())
        } else if k == "snaps" {
            (100, String::new())
        } else {
            (50, k.to_string())
        }
    };
    headers.sort_by_key(|(k, _)| rank(k));
    for (k, v) in headers.iter_mut() {
        if k == "snaps" || k == "validation-sets" {
            if let Node::List(l) = v {
                for e in l.iter_mut() {
                    if let Node::Obj(m) = e {
                        m.sort_by_key(|(f, _)| entry_rank(f));
                    }
                }
            }
        }
    }
    let mut all = vec![("type".to_string(), Node::Str(ty.into()))];
    all.extend(headers);
    let mut out = String::new();
    pretty(&Node::Obj(all), 0, &mut out);
    out.push('\n');
    out
}

// ---- read ----------------------------------------------------------------

/// A signed header value as the JSON a draft holds.
fn v_json(v: &assertion::V) -> Value {
    match v {
        assertion::V::S(s) => Value::String(s.clone()),
        assertion::V::L(l) => Value::Array(l.iter().map(v_json).collect()),
        assertion::V::M(m) => {
            Value::Object(m.iter().map(|(k, v)| (k.clone(), v_json(v))).collect())
        }
    }
}

/// The one model, system-user or validation-set in a signed stream, as a
/// draft.
fn parse_signed(bytes: &[u8]) -> Res<Map<String, Value>> {
    let stream = assertion::decode_stream(bytes).map_err(|e| (1, e.0))?;
    let ty = |a: &assertion::Assertion| match a.h.iter().find(|(k, _)| k == "type") {
        Some((_, assertion::V::S(t))) => t.clone(),
        _ => String::new(),
    };
    let mut ours = stream.iter().filter(|a| TYPES.contains(&ty(a).as_str()));
    let (Some(a), None) = (ours.next(), ours.next()) else {
        let seen: Vec<String> = stream.iter().map(ty).collect();
        let n = seen.iter().filter(|t| TYPES.contains(&t.as_str())).count();
        let what = if n == 0 {
            "no model, system-user or validation-set"
        } else {
            "more than one model, system-user or validation-set: give one"
        };
        return Err((1, format!("the stream holds {}: {what}", seen.join(", "))));
    };
    if !a.body.is_empty() {
        return Err((1, format!("the {} has a body, which it can't have", ty(a))));
    }
    Ok(a.h
        .iter()
        .filter(|(k, _)| k != "sign-key-sha3-384" && k != "body-length")
        .map(|(k, v)| (k.clone(), v_json(v)))
        .collect())
}

fn is_signed(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes)
        .trim_start()
        .starts_with("type:")
}

fn parse_draft(bytes: &[u8]) -> Res<Map<String, Value>> {
    if is_signed(bytes) {
        return parse_signed(bytes);
    }
    let text = String::from_utf8_lossy(bytes);
    let v: Value =
        serde_json::from_str(&text).map_err(|e| (1, format!("not a JSON draft: {e}")))?;
    match v {
        Value::Object(o) => Ok(o),
        _ => Err((1, "not a JSON draft: not an object".into())),
    }
}

fn draft_type(o: &Map<String, Value>) -> Res<&str> {
    match o.get("type").and_then(Value::as_str) {
        Some(t) if TYPES.contains(&t) => Ok(t),
        Some(t) => Err((
            1,
            format!(
                "a {t} assertion: only model, system-user and validation-set drafts are supported"
            ),
        )),
        None => Err((1, "the draft has no type".into())),
    }
}

/// A string safe in a record: no tab or newline.
fn plain(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.contains(['\t', '\n', '\r']))
}

pub fn read(bytes: &[u8]) -> Res<String> {
    let o = parse_draft(bytes)?;
    let ty = draft_type(&o)?;
    let mut out = format!("t\t{ty}\n");
    let keys: Vec<&str> = primary_keys(ty)
        .iter()
        .filter(|k| **k != "series")
        .map(|k| o.get(*k).and_then(plain).unwrap_or(""))
        .collect();
    out += &format!(
        "o\t{}\t{ty}\t{}\t{}\n",
        if is_signed(bytes) { "signed" } else { "draft" },
        keys.join("/"),
        o.get("revision").and_then(plain).unwrap_or("0")
    );
    let raw = |path: &str, v: &Value| format!("x\t{path}\t{v}\n");
    for (k, v) in &o {
        if k == "type" {
            continue;
        }
        match (k.as_str(), v) {
            ("snaps" | "validation-sets", Value::Array(items))
                if items
                    .iter()
                    .all(|e| e.get("name").and_then(plain).is_some()) =>
            {
                let tag = if k == "snaps" { "s" } else { "v" };
                for e in items {
                    let name = e.get("name").and_then(plain).unwrap_or_default();
                    out += &format!("{tag}\t{name}\n");
                    for (f, fv) in e.as_object().into_iter().flatten() {
                        if f == "name" {
                            continue;
                        }
                        match plain(fv) {
                            Some(s) => out += &format!("f\t{f}\t{s}\n"),
                            None => out += &raw(&format!("{k}[{name}].{f}"), fv),
                        }
                    }
                }
            }
            (_, Value::Array(items)) if items.iter().all(|i| plain(i).is_some()) => {
                for i in items {
                    out += &format!("l\t{k}\t{}\n", plain(i).unwrap_or_default());
                }
            }
            _ => match plain(v) {
                Some(s) => out += &format!("h\t{k}\t{s}\n"),
                None => out += &raw(k, v),
            },
        }
    }
    Ok(out)
}

// ---- check ---------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub struct Problem {
    pub error: bool,
    pub at: String,
    pub msg: String,
}

struct Report(Vec<Problem>);

impl Report {
    fn error(&mut self, at: &str, msg: impl Into<String>) {
        self.0.push(Problem {
            error: true,
            at: at.into(),
            msg: msg.into(),
        });
    }
    fn warning(&mut self, at: &str, msg: impl Into<String>) {
        self.0.push(Problem {
            error: false,
            at: at.into(),
            msg: msg.into(),
        });
    }
}

fn check_cmd(bytes: &[u8]) -> u8 {
    let problems = match parse_draft(bytes).and_then(|o| check(&o)) {
        Ok(p) => p,
        Err((_, msg)) => vec![Problem {
            error: true,
            at: "-".into(),
            msg,
        }],
    };
    let mut out = String::new();
    for p in &problems {
        out += &format!(
            "{}\t{}\t{}\n",
            if p.error { "error" } else { "warning" },
            p.at,
            p.msg.replace(['\t', '\n'], " ")
        );
    }
    let _ = std::io::stdout().lock().write_all(out.as_bytes());
    u8::from(problems.iter().any(|p| p.error))
}

/// snapd's snap names: 2-40 of a-z, 0-9 and `-`, a letter somewhere, no
/// leading, trailing or double `-` (snap/naming/validate.go).
pub fn valid_snap_name(n: &str) -> bool {
    (2..=40).contains(&n.len())
        && n.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && n.bytes().any(|b| b.is_ascii_lowercase())
        && !n.starts_with('-')
        && !n.ends_with('-')
        && !n.contains("--")
}

fn valid_snap_id(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// `^[a-zA-Z0-9](?:-?[a-zA-Z0-9])*$` (or lowercase only).
fn dashed_name(s: &str, upper: bool) -> bool {
    let ok =
        |b: u8| b.is_ascii_digit() || b.is_ascii_lowercase() || (upper && b.is_ascii_uppercase());
    !s.is_empty()
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.bytes().all(|b| ok(b) || b == b'-')
}

fn valid_username(s: &str) -> bool {
    let mut b = s.bytes();
    b.next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && b.all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'-' | b'.' | b'_')
        })
}

const RISKS: &[&str] = &["stable", "candidate", "beta", "edge"];

/// Whether a channel names a track, as snapd's channel.ParseVerbatim
/// reads it: `risk`, `risk/branch` don't; `track`, `track/risk`,
/// `track/risk/branch` do. Err for a malformed channel.
pub fn channel_has_track(ch: &str) -> Result<bool, String> {
    let parts: Vec<&str> = ch.split('/').collect();
    if parts.iter().any(|p| p.is_empty()) {
        return Err(format!("'{ch}' isn't a channel"));
    }
    match parts.as_slice() {
        [one] => Ok(!RISKS.contains(one)),
        [a, b] if RISKS.contains(a) => {
            let _ = b;
            Ok(false)
        }
        [_, b] if RISKS.contains(b) => Ok(true),
        [_, b] => Err(format!(
            "'{b}' in '{ch}' isn't a risk (stable, candidate, beta, edge)"
        )),
        [_, b, _] if RISKS.contains(b) => Ok(true),
        _ => Err(format!("'{ch}' isn't a channel")),
    }
}

/// RFC 3339 to Unix seconds.
pub fn rfc3339(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20
        || b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let num = |r: std::ops::Range<usize>| -> Option<i64> {
        let t = s.get(r)?;
        t.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| t.parse().ok())?
    };
    let (y, mo, d, h, mi, se) = (
        num(0..4)?,
        num(5..7)?,
        num(8..10)?,
        num(11..13)?,
        num(14..16)?,
        num(17..19)?,
    );
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    let mut rest = &s[19..];
    if let Some(r) = rest.strip_prefix('.') {
        let n = r.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return None;
        }
        rest = &r[n..];
    }
    let off = match rest {
        "Z" | "z" => 0,
        _ => {
            let rb = rest.as_bytes();
            if rb.len() != 6 || !matches!(rb[0], b'+' | b'-') || rb[3] != b':' {
                return None;
            }
            let oh: i64 = rest.get(1..3)?.parse().ok()?;
            let om: i64 = rest.get(4..6)?.parse().ok()?;
            let o = oh * 3600 + om * 60;
            if rb[0] == b'-' {
                -o
            } else {
                o
            }
        }
    };
    // days from civil (Howard Hinnant)
    let (y2, m2) = if mo <= 2 {
        (y - 1, mo + 9)
    } else {
        (y, mo - 3)
    };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + h * 3600 + mi * 60 + se - off)
}

/// The same instant one year later in the calendar (Go's AddDate(1,0,0)).
fn year_later(s: &str) -> Option<i64> {
    let y: i64 = s.get(0..4)?.parse().ok()?;
    let mut t = format!("{:04}{}", y + 1, &s[4..]);
    // 29 February plus a year is 1 March, as in Go
    if &t[5..10] == "02-29" {
        t.replace_range(5..10, "03-01");
    }
    rfc3339(&t)
}

fn valid_email(s: &str) -> bool {
    let addr = match (s.find('<'), s.strip_suffix('>')) {
        (Some(i), Some(_)) => &s[i + 1..s.len() - 1],
        _ => s,
    };
    let Some((local, domain)) = addr.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !domain.contains("..")
        && !addr
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '<' | '>' | ',' | ';'))
}

/// crypt(3): `$id$salt$hash` or `$id$rounds=N$salt$hash`, id ≥ 6.
pub fn check_password(pw: &str) -> Result<(), String> {
    let l: Vec<&str> = pw.splitn(5, '$').collect();
    if (l.len() != 4 && l.len() != 5) || !l[0].is_empty() {
        return Err("must be a crypt(3) hash ($<id>$<salt>$<hash>) of type 6 or higher".into());
    }
    let (id, rounds, salt, hash) = if l.len() == 4 {
        if l[2].starts_with("rounds=") {
            return Err("is missing its hash field".into());
        }
        (l[1], "", l[2], l[3])
    } else {
        (l[1], l[2], l[3], l[4])
    };
    match id.parse::<u32>() {
        Ok(n) if n >= 6 => {}
        Ok(_) => return Err("must be a crypt(3) hash of type 6 (sha512crypt) or higher".into()),
        Err(_) => {
            return Err("must be a crypt(3) hash ($<id>$<salt>$<hash>) of type 6 or higher".into())
        }
    }
    if let Some(r) = rounds.strip_prefix("rounds=") {
        match r.parse::<u64>() {
            Ok(n) if (5000..=999_999_999).contains(&n) => {}
            _ => return Err(format!("has rounds={r}, outside 5000-999999999")),
        }
    }
    let cs = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'/')
    };
    if !cs(salt) {
        return Err("has characters a crypt(3) salt can't hold".into());
    }
    if !cs(hash) {
        return Err("has characters a crypt(3) hash can't hold".into());
    }
    Ok(())
}

fn non_strings(v: &Value, at: &str, r: &mut Report) {
    match v {
        Value::String(_) => {}
        Value::Array(a) => {
            for (i, item) in a.iter().enumerate() {
                let name = item.get("name").and_then(Value::as_str);
                let at = match name {
                    Some(n) => format!("{at}[{n}]"),
                    None => format!("{at}[{i}]"),
                };
                non_strings(item, &at, r);
            }
        }
        Value::Object(o) => {
            for (k, item) in o {
                non_strings(item, &format!("{at}.{k}"), r);
            }
        }
        Value::Null => r.error(at, "is null: give a string, or leave it out"),
        other => r.error(
            at,
            format!("must be a string (\"{other}\"), as snap sign expects"),
        ),
    }
}

fn s<'a>(o: &'a Map<String, Value>, k: &str) -> Option<&'a str> {
    o.get(k).and_then(Value::as_str)
}

fn need(o: &Map<String, Value>, k: &str, r: &mut Report) -> Option<String> {
    match o.get(k) {
        None => {
            r.error(k, "is required");
            None
        }
        Some(Value::String(v)) if v.is_empty() => {
            r.error(k, "is empty");
            None
        }
        Some(Value::String(v)) => Some(v.clone()),
        Some(_) => None, // reported as a non-string
    }
}

fn int_header(o: &Map<String, Value>, k: &str, r: &mut Report) -> Option<i64> {
    let v = s(o, k)?;
    match v.parse::<i64>() {
        Ok(n) if v.bytes().all(|b| b.is_ascii_digit()) => Some(n),
        _ => {
            r.error(k, format!("must be a whole number, not '{v}'"));
            None
        }
    }
}

fn str_list(o: &Map<String, Value>, k: &str, r: &mut Report) -> Vec<String> {
    match o.get(k) {
        None => Vec::new(),
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        Some(_) => {
            r.error(k, "must be a list");
            Vec::new()
        }
    }
}

fn time_header(o: &Map<String, Value>, k: &str, r: &mut Report) -> Option<i64> {
    let v = need(o, k, r)?;
    let t = rfc3339(&v);
    if t.is_none() {
        r.error(
            k,
            format!("must be an RFC 3339 time (e.g. 2026-11-27T10:00:00Z), not '{v}'"),
        );
    }
    t
}

pub fn check(o: &Map<String, Value>) -> Res<Vec<Problem>> {
    let ty = draft_type(o)?;
    let mut r = Report(Vec::new());
    for (k, v) in o {
        non_strings(v, k, &mut r);
    }
    need(o, "authority-id", &mut r);
    if o.contains_key("format") {
        int_header(o, "format", &mut r);
    }
    if o.contains_key("revision") {
        int_header(o, "revision", &mut r);
    }
    match ty {
        "model" => check_model(o, &mut r),
        "system-user" => check_system_user(o, &mut r),
        _ => check_validation_set(o, &mut r),
    }
    Ok(r.0)
}

fn check_model(o: &Map<String, Value>, r: &mut Report) {
    need(o, "series", r);
    let brand = need(o, "brand-id", r);
    if let (Some(a), Some(b)) = (s(o, "authority-id"), brand.as_deref()) {
        if a != b {
            r.error(
                "authority-id",
                format!("must equal brand-id ('{a}' isn't '{b}'): models are signed by the brand"),
            );
        }
    }
    if let Some(m) = need(o, "model", r) {
        if m.bytes().any(|b| b.is_ascii_uppercase()) {
            r.error("model", "can't hold uppercase letters");
        } else if !dashed_name(&m, true) {
            r.error(
                "model",
                format!("'{m}' isn't a model name: letters and digits, single '-' between them"),
            );
        }
    }
    time_header(o, "timestamp", r);
    let classic = s(o, "classic") == Some("true");
    if let Some(c) = s(o, "classic") {
        if c != "true" && c != "false" {
            r.error("classic", "must be 'true' or 'false'");
        }
    }
    // distribution: an os-release ID, in a classic model with snaps only
    match (classic && o.contains_key("snaps"), s(o, "distribution")) {
        (true, None) | (true, Some("")) => r.error(
            "distribution",
            "is required in a classic model (an os-release ID, e.g. ubuntu)",
        ),
        (true, Some(d))
            if !d
                .bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-')) =>
        {
            r.error(
                "distribution",
                format!(
                    "'{d}' isn't an os-release ID: lowercase letters, digits, '.', '_' and '-'"
                ),
            )
        }
        (false, Some(_)) => r.error(
            "distribution",
            "is only for a classic model with the extended snaps header",
        ),
        _ => {}
    }
    let Some(snaps) = o.get("snaps") else {
        // classic and pre-UC20 models: the old headers
        for h in ["grade", "storage-safety"] {
            if o.contains_key(h) {
                r.error(h, "needs the extended snaps header");
            }
        }
        if classic {
            for h in ["kernel", "base"] {
                if o.contains_key(h) {
                    r.error(
                        h,
                        "can't be given in a classic model without the extended snaps header",
                    );
                }
            }
        } else {
            for h in ["architecture", "gadget", "kernel"] {
                need(o, h, r);
            }
        }
        return;
    };
    for h in ["gadget", "kernel", "required-snaps"] {
        if o.contains_key(h) {
            r.error(
                h,
                "can't be given with the snaps header: list the snap in snaps instead",
            );
        }
    }
    need(o, "architecture", r);
    let base = need(o, "base", r).unwrap_or_default();
    if !base.is_empty() && !valid_snap_name(&base) {
        r.error("base", format!("'{base}' isn't a snap name"));
    }
    let grade = s(o, "grade").unwrap_or("signed");
    if !["dangerous", "signed", "secured"].contains(&grade) {
        r.error(
            "grade",
            format!("must be dangerous, signed or secured, not '{grade}'"),
        );
    }
    if let Some(ss) = s(o, "storage-safety") {
        if !["encrypted", "prefer-encrypted", "prefer-unencrypted"].contains(&ss) {
            r.error(
                "storage-safety",
                format!("must be encrypted, prefer-encrypted or prefer-unencrypted, not '{ss}'"),
            );
        } else if grade == "secured" && ss != "encrypted" {
            r.error(
                "storage-safety",
                "can only be 'encrypted' in a secured model",
            );
        }
    }
    let Some(list) = snaps.as_array() else {
        r.error("snaps", "must be a list of snaps");
        return;
    };
    let (mut gadgets, mut kernels, mut snapds) = (Vec::new(), Vec::new(), Vec::new());
    let mut names: Vec<String> = Vec::new();
    let mut ids: Vec<(String, String)> = Vec::new();
    for (i, e) in list.iter().enumerate() {
        let Some(e) = e.as_object() else {
            r.error(&format!("snaps[{i}]"), "must be a snap entry (an object)");
            continue;
        };
        let Some(name) = s(e, "name").filter(|n| !n.is_empty()) else {
            r.error(&format!("snaps[{i}]"), "has no name");
            continue;
        };
        let at = |f: &str| format!("snaps[{name}].{f}");
        if !valid_snap_name(name) {
            r.error(
                &format!("snaps[{name}]"),
                format!("'{name}' isn't a snap name"),
            );
        }
        if names.iter().any(|n| n == name) {
            r.error(&format!("snaps[{name}]"), "is listed twice");
        }
        names.push(name.into());
        match s(e, "id") {
            Some(id) if !valid_snap_id(id) => r.error(
                &at("id"),
                format!("'{id}' isn't a snap ID (32 letters and digits)"),
            ),
            Some(id) => {
                if let Some((other, _)) = ids.iter().find(|(_, i)| i == id) {
                    r.error(&at("id"), format!("is also the ID of {other}"));
                }
                ids.push((name.into(), id.into()));
            }
            None if !e.contains_key("id") && grade != "dangerous" => {
                r.error(&at("id"), format!("is mandatory in a {grade} model (only dangerous models may list a snap without one)"));
            }
            None => {}
        }
        let ty = s(e, "type").unwrap_or("app");
        if !["app", "base", "gadget", "kernel", "core", "snapd"].contains(&ty) {
            r.error(
                &at("type"),
                format!("must be app, base, gadget, kernel, core or snapd, not '{ty}'"),
            );
        }
        match ty {
            "gadget" => gadgets.push(name.to_string()),
            "kernel" => kernels.push(name.to_string()),
            "snapd" => snapds.push(name.to_string()),
            _ => {}
        }
        if name == base && ty != "base" {
            r.error(
                &at("type"),
                format!("must be 'base' for the model's base, not '{ty}'"),
            );
        }
        let essential = matches!(ty, "snapd" | "kernel" | "gadget") || name == base;
        match s(e, "presence") {
            Some(_) if essential => r.error(
                &at("presence"),
                "can't be given for an essential snap (snapd, kernel, gadget or the base)",
            ),
            Some(p) if p != "required" && p != "optional" => r.error(
                &at("presence"),
                format!("must be required or optional, not '{p}'"),
            ),
            _ => {}
        }
        if e.contains_key("modes") && essential {
            r.error(
                &at("modes"),
                "can't be given for an essential snap: it is in every mode",
            );
        }
        if let Some(ch) = s(e, "default-channel") {
            match channel_has_track(ch) {
                Ok(true) => {}
                Ok(false) => r.error(
                    &at("default-channel"),
                    format!("must give a track ('{ch}' has none: e.g. latest/{ch}, or 24/{ch} for a UC24 gadget or kernel)"),
                ),
                Err(m) => r.error(&at("default-channel"), m),
            }
        }
        if s(e, "classic") == Some("true") {
            if !classic {
                r.error(
                    &at("classic"),
                    "only a classic model can have classic snaps",
                );
            } else if ty != "app" {
                r.error(&at("classic"), "only an app snap can be classic");
            }
        }
    }
    if classic && !kernels.is_empty() && gadgets.is_empty() {
        r.error(
            "snaps",
            "a classic model can list a kernel only with a gadget",
        );
    }
    if !classic {
        if gadgets.is_empty() {
            r.error(
                "snaps",
                "must list the model's gadget (a snap of type gadget)",
            );
        }
        if kernels.is_empty() {
            r.error(
                "snaps",
                "must list the model's kernel (a snap of type kernel)",
            );
        }
    }
    for (kind, l) in [
        ("gadget", &gadgets),
        ("kernel", &kernels),
        ("snapd", &snapds),
    ] {
        if l.len() > 1 {
            r.error(
                "snaps",
                format!("lists more than one {kind}: {}", l.join(", ")),
            );
        }
    }
    if let Some(vs) = o.get("validation-sets") {
        check_model_validation_sets(vs, r);
    }
    for h in ["serial-authority", "system-user-authority"] {
        match o.get(h) {
            Some(Value::String(v)) if h == "system-user-authority" && v == "*" => {}
            Some(Value::Array(_)) | None => {}
            Some(_) => r.error(h, "must be a list of account IDs"),
        }
    }
}

fn check_model_validation_sets(vs: &Value, r: &mut Report) {
    let Some(list) = vs.as_array() else {
        r.error("validation-sets", "must be a list of validation sets");
        return;
    };
    let mut seen = Vec::new();
    for (i, e) in list.iter().enumerate() {
        let Some(e) = e.as_object() else {
            r.error(&format!("validation-sets[{i}]"), "must be an object");
            continue;
        };
        let name = s(e, "name").unwrap_or_default();
        let at = |f: &str| {
            format!(
                "validation-sets[{}].{f}",
                if name.is_empty() {
                    i.to_string()
                } else {
                    name.to_string()
                }
            )
        };
        if name.is_empty() {
            r.error(&at("name"), "is required");
        } else if !dashed_name(name, false) {
            r.error(&at("name"), format!("'{name}' isn't a validation set name"));
        }
        let acct = s(e, "account-id").unwrap_or_default();
        if acct.is_empty() {
            r.error(&at("account-id"), "is required");
        }
        let key = format!("{acct}/{name}");
        if seen.contains(&key) {
            r.error(&at("name"), format!("{key} is listed twice"));
        }
        seen.push(key);
        if let Some(seq) = s(e, "sequence") {
            if seq.parse::<u64>().map_or(true, |n| n == 0) {
                r.error(
                    &at("sequence"),
                    format!(
                        "must be a number above 0, or left out to track the latest, not '{seq}'"
                    ),
                );
            }
        }
        match s(e, "mode") {
            Some("enforce" | "prefer-enforce") => {}
            Some(m) => r.error(
                &at("mode"),
                format!("must be enforce or prefer-enforce, not '{m}'"),
            ),
            None => r.error(&at("mode"), "is required (enforce or prefer-enforce)"),
        }
    }
}

fn check_system_user(o: &Map<String, Value>, r: &mut Report) {
    need(o, "brand-id", r);
    if let Some(e) = need(o, "email", r) {
        if !valid_email(&e) {
            r.error("email", format!("'{e}' isn't an RFC 5322 email address"));
        }
    }
    str_list(o, "series", r);
    let models = str_list(o, "models", r);
    let serials = str_list(o, "serials", r);
    let format = s(o, "format")
        .and_then(|f| f.parse::<u32>().ok())
        .unwrap_or(0);
    if !serials.is_empty() {
        if format < 1 {
            r.error("format", "must be at least 1 with serials");
        }
        if models.len() != 1 {
            r.error("serials", "needs exactly one model in models");
        }
    }
    if let Some(u) = need(o, "username", r) {
        if !valid_username(&u) {
            r.error("username", format!("'{u}' isn't a username: a lowercase letter or digit, then lowercase letters, digits, '-', '.' or '_'"));
        }
    }
    let pw = s(o, "password").unwrap_or_default();
    if !pw.is_empty() {
        if let Err(m) = check_password(pw) {
            r.error("password", m);
        }
    }
    if let Some(f) = s(o, "force-password-change") {
        if f != "true" && f != "false" {
            r.error("force-password-change", "must be 'true' or 'false'");
        } else if f == "true" && pw.is_empty() {
            r.error("force-password-change", "needs a password");
        }
    }
    let keys = str_list(o, "ssh-keys", r);
    if keys.is_empty() && pw.is_empty() {
        r.warning(
            "ssh-keys",
            "no SSH keys and no password: the user can't log in",
        );
    }
    let since = time_header(o, "since", r);
    let until = time_header(o, "until", r);
    if let (Some(a), Some(b)) = (since, until) {
        if b < a {
            r.error("until", "can't be before since");
        } else if models.is_empty() && s(o, "since").and_then(year_later).is_some_and(|y| b > y) {
            r.error(
                "until",
                "can be at most 365 days after since when no models are given",
            );
        }
    }
    if let Some(p) = s(o, "user-presence") {
        if format < 2 {
            r.error("format", "must be at least 2 with user-presence");
        }
        if p != "until-expiration" {
            r.error(
                "user-presence",
                format!("can only be until-expiration, not '{p}'"),
            );
        }
    }
}

fn check_validation_set(o: &Map<String, Value>, r: &mut Report) {
    need(o, "series", r);
    let acct = need(o, "account-id", r);
    if let (Some(a), Some(b)) = (s(o, "authority-id"), acct.as_deref()) {
        if a != b {
            r.error(
                "authority-id",
                format!("must equal account-id ('{a}' isn't '{b}'): validation sets are signed by their account"),
            );
        }
    }
    if let Some(n) = need(o, "name", r) {
        if !dashed_name(&n, false) {
            r.error("name", format!("'{n}' isn't a validation set name: lowercase letters and digits, single '-' between them"));
        }
    }
    if need(o, "sequence", r).is_some() {
        if let Some(n) = int_header(o, "sequence", r) {
            if n < 1 {
                r.error("sequence", "must be 1 or more");
            }
        }
    }
    time_header(o, "timestamp", r);
    let Some(snaps) = o.get("snaps") else {
        r.error("snaps", "is required");
        return;
    };
    let Some(list) = snaps.as_array() else {
        r.error("snaps", "must be a list of snaps");
        return;
    };
    let mut names: Vec<&str> = Vec::new();
    let mut ids: Vec<(&str, &str)> = Vec::new();
    for (i, e) in list.iter().enumerate() {
        let Some(e) = e.as_object() else {
            r.error(&format!("snaps[{i}]"), "must be a snap entry (an object)");
            continue;
        };
        let Some(name) = s(e, "name").filter(|n| !n.is_empty()) else {
            r.error(&format!("snaps[{i}]"), "has no name");
            continue;
        };
        let at = |f: &str| format!("snaps[{name}].{f}");
        if !valid_snap_name(name) {
            r.error(
                &format!("snaps[{name}]"),
                format!("'{name}' isn't a snap name"),
            );
        }
        if names.contains(&name) {
            r.error(&format!("snaps[{name}]"), "is listed twice");
        }
        names.push(name);
        match s(e, "id") {
            None => r.error(&at("id"), "is required"),
            Some(id) if !valid_snap_id(id) => r.error(
                &at("id"),
                format!("'{id}' isn't a snap ID (32 letters and digits)"),
            ),
            Some(id) => {
                if let Some((other, _)) = ids.iter().find(|(_, i)| *i == id) {
                    r.error(&at("id"), format!("is also the ID of {other}"));
                }
                ids.push((name, id));
            }
        }
        let presence = s(e, "presence").unwrap_or("required");
        if !["required", "optional", "invalid"].contains(&presence) {
            r.error(
                &at("presence"),
                format!("must be required, optional or invalid, not '{presence}'"),
            );
        }
        if let Some(rev) = s(e, "revision") {
            match rev.parse::<u64>() {
                Ok(n) if n > 0 && rev.bytes().all(|b| b.is_ascii_digit()) => {
                    if presence == "invalid" {
                        r.error(
                            &at("revision"),
                            "can't be given for a snap whose presence is invalid",
                        );
                    }
                }
                _ => r.error(
                    &at("revision"),
                    format!("must be a revision number above 0, not '{rev}'"),
                ),
            }
        }
        if let Some(c) = e.get("components") {
            let snap_rev = s(e, "revision").is_some_and(|r| r.parse::<u64>().is_ok_and(|n| n > 0));
            check_vs_components(name, c, snap_rev, r);
        }
    }
}

/// A validation set snap's components, as snapd checks them: each a
/// presence string, or an object with presence and an optional revision;
/// no revision when invalid, and one exactly when the snap has one.
fn check_vs_components(snap: &str, c: &Value, snap_rev: bool, r: &mut Report) {
    let Some(map) = c.as_object() else {
        r.error(
            &format!("snaps[{snap}].components"),
            "must be a map of components",
        );
        return;
    };
    for (name, comp) in map {
        let at = format!("snaps[{snap}].components.{name}");
        let (presence, rev) = match comp {
            Value::String(p) => (p.as_str(), None),
            Value::Object(o) => (s(o, "presence").unwrap_or("required"), s(o, "revision")),
            _ => {
                r.error(&at, "must be a presence or an object");
                continue;
            }
        };
        if !valid_snap_name(name) {
            r.error(&at, format!("'{name}' isn't a component name"));
        }
        if !["required", "optional", "invalid"].contains(&presence) {
            r.error(
                &format!("{at}.presence"),
                format!("must be required, optional or invalid, not '{presence}'"),
            );
        }
        match rev {
            Some(v) if !v.parse::<u64>().is_ok_and(|n| n > 0) => r.error(
                &format!("{at}.revision"),
                format!("must be a revision number above 0, not '{v}'"),
            ),
            Some(_) if presence == "invalid" => r.error(
                &format!("{at}.revision"),
                "can't be given for a component whose presence is invalid",
            ),
            Some(_) if !snap_rev => r.error(
                &format!("{at}.revision"),
                format!("can't be given when {snap} has no revision"),
            ),
            None if snap_rev && presence != "invalid" => r.error(
                &format!("{at}.revision"),
                format!("is required since {snap} has a revision"),
            ),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const UC24_AMD64: &str = include_str!("testdata/models/ubuntu-core-24-amd64.json");
    const UC24_PI: &str = include_str!("testdata/models/ubuntu-core-24-pi-arm64.json");
    const UC24_SIGNED: &str = include_str!("testdata/models/ubuntu-core-24-amd64.model");

    fn obj(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    fn round_trip(text: &str) -> Value {
        let ty = serde_json::from_str::<Value>(text).unwrap()["type"]
            .as_str()
            .unwrap()
            .to_string();
        let records = read(text.as_bytes()).unwrap();
        serde_json::from_str(&write(&ty, &records).unwrap()).unwrap()
    }

    #[test]
    fn canonical_models_round_trip() {
        for m in [UC24_AMD64, UC24_PI] {
            let v: Value = serde_json::from_str(m).unwrap();
            assert_eq!(round_trip(m), v);
            assert!(check(v.as_object().unwrap()).unwrap().is_empty(), "{m}");
        }
    }

    #[test]
    fn header_order_and_indent() {
        let out = write(
            "model",
            "h\tmodel\tkiosk\nh\tbrand-id\tabc\nh\tauthority-id\tabc\nh\tseries\t16\ns\tpc\nf\tid\tX\nf\ttype\tgadget\nh\tarchitecture\tamd64\n",
        )
        .unwrap();
        assert_eq!(
            out,
            "{\n  \"type\": \"model\",\n  \"authority-id\": \"abc\",\n  \"series\": \"16\",\n  \"brand-id\": \"abc\",\n  \"model\": \"kiosk\",\n  \"architecture\": \"amd64\",\n  \"snaps\": [\n    {\n      \"name\": \"pc\",\n      \"type\": \"gadget\",\n      \"id\": \"X\"\n    }\n  ]\n}\n"
        );
    }

    #[test]
    fn strings_survive() {
        let name = "Kiosk \"Pro\" \\ ünïcødé ✓";
        let out = write("model", &format!("h\tdisplay-name\t{name}\n")).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["display-name"], name);
        assert_eq!(round_trip(&out), v);
    }

    #[test]
    fn unmodelled_fields_kept() {
        let d = json!({"type": "model", "authority-id": "a", "brand-id": "a", "series": "16",
            "snaps": [{"name": "app", "modes": ["run", "ephemeral"],
                       "components": {"c1": "required", "c2": {"presence": "optional", "modes": ["run"]}}}],
            "body": "line one\nline two", "odd": 3});
        assert_eq!(round_trip(&d.to_string()), d);
    }

    #[test]
    fn refused_records() {
        assert!(write("model", "h\tdisplay-name\ta\tb\n").is_err());
        assert!(write("model", "f\tname\tx\n").is_err());
        assert!(write("model", "t\tsystem-user\n").is_err());
        assert!(write("model", "q\tx\n").is_err());
        assert!(write("model", "h\ttype\tsystem-user\n").is_err());
    }

    /// A signed-looking assertion: headers, a blank line, a dummy signature.
    fn fake_signed(headers: &str) -> String {
        format!("{headers}\nsign-key-sha3-384: abc\n\nAcLBUgQAAQoABgUC\n")
    }

    #[test]
    fn signed_model_read() {
        let records = read(UC24_SIGNED.as_bytes()).unwrap();
        assert!(records.starts_with("t\tmodel\no\tsigned\tmodel\tcanonical/ubuntu-core-24-amd64\t"));
        let v: Value = serde_json::from_str(&write("model", &records).unwrap()).unwrap();
        assert!(v.get("sign-key-sha3-384").is_none());
        assert!(v.get("body-length").is_none());
        // the signed model holds what its draft does, as strings
        let draft: Value = serde_json::from_str(UC24_AMD64).unwrap();
        for k in ["brand-id", "model", "architecture", "base", "grade"] {
            assert_eq!(v[k], draft[k], "{k}");
        }
        let names = |x: &Value| -> Vec<String> {
            x["snaps"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["name"].as_str().unwrap().to_string())
                .collect()
        };
        assert_eq!(names(&v), names(&draft));
        assert!(v["snaps"][0]["id"].is_string());
        assert!(check(v.as_object().unwrap())
            .unwrap()
            .iter()
            .all(|p| !p.error));
        // a draft says so
        assert!(read(UC24_AMD64.as_bytes())
            .unwrap()
            .contains("\no\tdraft\tmodel\tcanonical/ubuntu-core-24-amd64\t"));
    }

    #[test]
    fn signed_streams() {
        let account = fake_signed("type: account\nauthority-id: canonical\naccount-id: x");
        let key = fake_signed("type: account-key\nauthority-id: canonical\naccount-id: x");
        let vs = fake_signed(
            "type: validation-set\nauthority-id: x\nseries: 16\naccount-id: x\nname: base\nsequence: 3\nrevision: 2\nsnaps:\n  -\n    name: core24\n    id: abc\n    presence: required\n    revision: 2307",
        );
        let r = read(format!("{account}\n{key}\n{vs}").as_bytes()).unwrap();
        assert!(
            r.starts_with("t\tvalidation-set\no\tsigned\tvalidation-set\tx/base/3\t2\n"),
            "{r}"
        );
        assert!(r.contains("s\tcore24\n") && r.contains("f\trevision\t2307\n"));
        let e = read(format!("{account}\n{key}").as_bytes()).unwrap_err();
        assert!(
            e.1.contains("holds account, account-key: no model"),
            "{}",
            e.1
        );
        let e = read(format!("{vs}\n{vs}").as_bytes()).unwrap_err();
        assert!(e.1.contains("more than one"), "{}", e.1);
        let body = "type: model\nauthority-id: x\nbrand-id: x\nseries: 16\nmodel: m\nbody-length: 2\nsign-key-sha3-384: abc\n\nhi\n\nAcLBUgQAAQoABgUC\n";
        assert!(read(body.as_bytes()).unwrap_err().1.contains("has a body"));
    }

    #[test]
    fn foreign_input() {
        let e = read(fake_signed("type: serial\nauthority-id: x").as_bytes()).unwrap_err();
        assert!(
            e.1.contains("no model, system-user or validation-set"),
            "{}",
            e.1
        );
        let e = read(br#"{"type": "serial"}"#).unwrap_err();
        assert!(e.1.contains("only model, system-user and validation-set"));
        assert!(read(b"[1]").is_err());
    }

    fn errors(v: Value) -> Vec<(String, String)> {
        check(&obj(v))
            .unwrap()
            .into_iter()
            .filter(|p| p.error)
            .map(|p| (p.at, p.msg))
            .collect()
    }

    fn model() -> Value {
        serde_json::from_str(UC24_AMD64).unwrap()
    }

    fn has(errs: &[(String, String)], at: &str, needle: &str) -> bool {
        errs.iter().any(|(a, m)| a == at && m.contains(needle))
    }

    #[test]
    fn model_rules() {
        let mut m = model();
        m["snaps"][0]["default-channel"] = json!("stable");
        assert!(has(
            &errors(m),
            "snaps[pc].default-channel",
            "must give a track"
        ));

        let mut m = model();
        m["snaps"][4].as_object_mut().unwrap().remove("id");
        assert!(has(
            &errors(m.clone()),
            "snaps[console-conf].id",
            "mandatory in a signed model"
        ));
        m["grade"] = json!("dangerous");
        assert!(errors(m).is_empty());

        let mut m = model();
        m["authority-id"] = json!("other");
        assert!(has(&errors(m), "authority-id", "must equal brand-id"));

        let mut m = model();
        m["model"] = json!("Kiosk");
        assert!(has(&errors(m), "model", "uppercase"));
        let mut m = model();
        m["model"] = json!("kiosk--x");
        assert!(has(&errors(m), "model", "isn't a model name"));

        let mut m = model();
        m["grade"] = json!("insecure");
        assert!(has(&errors(m), "grade", "dangerous, signed or secured"));
        let mut m = model();
        m["grade"] = json!("secured");
        m["storage-safety"] = json!("prefer-encrypted");
        assert!(has(&errors(m), "storage-safety", "only be 'encrypted'"));

        let mut m = model();
        m["snaps"][0]["presence"] = json!("optional");
        assert!(has(&errors(m), "snaps[pc].presence", "essential"));
        let mut m = model();
        m["snaps"][2]["modes"] = json!(["run"]);
        assert!(has(&errors(m), "snaps[core24].modes", "essential"));
        let mut m = model();
        m["snaps"][4]["presence"] = json!("maybe");
        assert!(has(
            &errors(m),
            "snaps[console-conf].presence",
            "required or optional"
        ));
        let mut m = model();
        m["snaps"][4]["type"] = json!("os");
        assert!(has(&errors(m), "snaps[console-conf].type", "app, base"));

        let mut m = model();
        m["snaps"].as_array_mut().unwrap().remove(1);
        assert!(has(&errors(m), "snaps", "kernel"));
        let mut m = model();
        let k = m["snaps"][1].clone();
        let mut k2 = k.clone();
        k2["name"] = json!("other-kernel");
        m["snaps"].as_array_mut().unwrap().push(k2);
        let e = errors(m.clone());
        assert!(has(&e, "snaps", "more than one kernel"));
        assert!(has(
            &e,
            "snaps[other-kernel].id",
            "also the ID of pc-kernel"
        ));
        m["snaps"].as_array_mut().unwrap().push(k);
        assert!(has(&errors(m), "snaps[pc-kernel]", "listed twice"));

        let mut m = model();
        m.as_object_mut().unwrap().remove("base");
        assert!(has(&errors(m), "base", "required"));
        let mut m = model();
        m["kernel"] = json!("pc-kernel");
        assert!(has(
            &errors(m),
            "kernel",
            "can't be given with the snaps header"
        ));

        let mut m = model();
        m["validation-sets"] =
            json!([{"account-id": "a", "name": "s", "sequence": "0", "mode": "strict"}]);
        let e = errors(m);
        assert!(has(&e, "validation-sets[s].sequence", "above 0"));
        assert!(has(
            &e,
            "validation-sets[s].mode",
            "enforce or prefer-enforce"
        ));

        let mut m = model();
        m["revision"] = json!(3);
        assert!(has(&errors(m), "revision", "must be a string"));
        let mut m = model();
        m["snaps"][0]["id"] = json!(true);
        assert!(has(&errors(m), "snaps[pc].id", "must be a string"));
        let mut m = model();
        m["timestamp"] = json!("yesterday");
        assert!(has(&errors(m), "timestamp", "RFC 3339"));
    }

    fn user() -> Value {
        json!({"type": "system-user", "authority-id": "abc", "brand-id": "abc",
               "email": "dev@example.com", "series": ["16"], "models": ["kiosk"],
               "username": "dev", "name": "Dev",
               "ssh-keys": ["ssh-ed25519 AAAA dev@host"],
               "since": "2026-11-27T10:00:00Z", "until": "2027-11-27T10:00:00Z"})
    }

    #[test]
    fn system_user_rules() {
        assert!(errors(user()).is_empty());
        let mut u = user();
        u["models"] = json!([]);
        u["until"] = json!("2027-12-31T10:00:00Z");
        assert!(has(&errors(u.clone()), "until", "365 days"));
        u["until"] = json!("2027-11-27T10:00:00Z");
        assert!(errors(u.clone()).is_empty());
        u["until"] = json!("2026-11-26T10:00:00Z");
        assert!(has(&errors(u), "until", "before since"));

        let mut u = user();
        u["password"] = json!("hunter2");
        assert!(has(&errors(u.clone()), "password", "crypt(3) hash"));
        u["password"] = json!("$1$salt$hash");
        assert!(has(&errors(u.clone()), "password", "type 6"));
        u["password"] = json!("$6$rounds=100$salt$hash");
        assert!(has(&errors(u.clone()), "password", "rounds"));
        u["password"] = json!("$6$sa!t$hash");
        assert!(has(&errors(u.clone()), "password", "salt"));
        u["password"] = json!("$6$rounds=5000$saltsalt$abcdefghijklmnopqrstuvwxyz./0123456789");
        assert!(errors(u).is_empty());

        let mut u = user();
        u["force-password-change"] = json!("true");
        assert!(has(&errors(u), "force-password-change", "needs a password"));

        let mut u = user();
        u["serials"] = json!(["SN123"]);
        assert!(has(&errors(u.clone()), "format", "at least 1"));
        u["format"] = json!("1");
        assert!(errors(u.clone()).is_empty());
        u["models"] = json!(["a", "b"]);
        assert!(has(&errors(u), "serials", "exactly one model"));

        let mut u = user();
        u["user-presence"] = json!("until-expiration");
        assert!(has(&errors(u.clone()), "format", "at least 2"));
        u["format"] = json!("2");
        assert!(errors(u.clone()).is_empty());
        u["user-presence"] = json!("forever");
        assert!(has(&errors(u), "user-presence", "until-expiration"));

        let mut u = user();
        u["username"] = json!("Dev");
        assert!(has(&errors(u), "username", "isn't a username"));
        let mut u = user();
        u["email"] = json!("not an email");
        assert!(has(&errors(u), "email", "RFC 5322"));
        let mut u = user();
        u["ssh-keys"] = json!([]);
        let p = check(&obj(u)).unwrap();
        assert!(p.iter().any(|p| !p.error && p.at == "ssh-keys"));
        assert!(p.iter().all(|p| !p.error));
    }

    fn vset() -> Value {
        json!({"type": "validation-set", "authority-id": "abc", "account-id": "abc",
               "series": "16", "name": "base-set", "sequence": "4",
               "timestamp": "2026-11-27T10:00:00Z",
               "snaps": [{"name": "core24", "id": "dwTAh7MZZ01zyriOZErqd1JynQLiOGvM",
                          "presence": "required", "revision": "2307"}]})
    }

    #[test]
    fn validation_set_rules() {
        assert!(errors(vset()).is_empty());
        let mut v = vset();
        v["authority-id"] = json!("x");
        assert!(has(&errors(v), "authority-id", "account-id"));
        let mut v = vset();
        v["name"] = json!("Base_Set");
        assert!(has(&errors(v), "name", "isn't a validation set name"));
        let mut v = vset();
        v["sequence"] = json!("0");
        assert!(has(&errors(v), "sequence", "1 or more"));
        let mut v = vset();
        v["snaps"][0]["presence"] = json!("invalid");
        assert!(has(&errors(v), "snaps[core24].revision", "invalid"));
        let mut v = vset();
        v["snaps"][0]["revision"] = json!("-1");
        assert!(has(&errors(v), "snaps[core24].revision", "above 0"));
        let mut v = vset();
        v["snaps"][0].as_object_mut().unwrap().remove("id");
        assert!(has(&errors(v), "snaps[core24].id", "required"));
        let mut v = vset();
        v.as_object_mut().unwrap().remove("snaps");
        assert!(has(&errors(v), "snaps", "required"));
        let mut v = vset();
        v.as_object_mut().unwrap().remove("timestamp");
        assert!(has(&errors(v), "timestamp", "required"));
        let mut v = vset();
        v["snaps"][0]["presence"] = json!("sometimes");
        assert!(has(
            &errors(v),
            "snaps[core24].presence",
            "required, optional or invalid"
        ));
    }

    #[test]
    fn classic_model_rules() {
        let classic = || {
            let mut m = model();
            m["classic"] = json!("true");
            m["distribution"] = json!("ubuntu");
            m
        };
        assert!(errors(classic()).is_empty());
        let mut m = classic();
        m.as_object_mut().unwrap().remove("distribution");
        assert!(has(
            &errors(m),
            "distribution",
            "required in a classic model"
        ));
        let mut m = classic();
        m["distribution"] = json!("Ubuntu");
        assert!(has(&errors(m), "distribution", "os-release ID"));
        let mut m = model();
        m["distribution"] = json!("ubuntu");
        assert!(has(&errors(m), "distribution", "only for a classic model"));
        // a kernel needs a gadget; neither is fine
        let mut m = classic();
        let snaps: Vec<Value> = m["snaps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["type"] != "gadget")
            .cloned()
            .collect();
        m["snaps"] = json!(snaps);
        assert!(has(&errors(m), "snaps", "kernel only with a gadget"));
        let mut m = classic();
        let snaps: Vec<Value> = m["snaps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["type"] != "gadget" && e["type"] != "kernel")
            .cloned()
            .collect();
        m["snaps"] = json!(snaps);
        assert!(errors(m).is_empty());
        // old style: no snaps
        let old = json!({"type": "model", "authority-id": "acme", "brand-id": "acme",
            "series": "16", "model": "desk", "classic": "true",
            "timestamp": "2026-11-27T10:00:00Z"});
        assert!(errors(old.clone()).is_empty());
        let mut m = old.clone();
        m["kernel"] = json!("pc-kernel");
        assert!(has(
            &errors(m),
            "kernel",
            "without the extended snaps header"
        ));
        let mut m = old.clone();
        m["base"] = json!("core24");
        assert!(has(&errors(m), "base", "without the extended snaps header"));
        let mut m = old;
        m["distribution"] = json!("ubuntu");
        assert!(has(&errors(m), "distribution", "only for a classic model"));
    }

    #[test]
    fn validation_set_components() {
        let with = |comps: Value, rev: bool| {
            let mut v = vset();
            if !rev {
                v["snaps"][0].as_object_mut().unwrap().remove("revision");
            }
            v["snaps"][0]["components"] = comps;
            errors(v)
        };
        assert!(with(
            json!({"extra": {"presence": "required", "revision": "3"}}),
            true
        )
        .is_empty());
        assert!(with(json!({"extra": "optional"}), false).is_empty());
        assert!(with(json!({"extra": "invalid"}), true).is_empty());
        assert!(has(
            &with(json!({"extra": "required"}), true),
            "snaps[core24].components.extra.revision",
            "required since core24 has a revision"
        ));
        assert!(has(
            &with(
                json!({"extra": {"presence": "invalid", "revision": "3"}}),
                true
            ),
            "snaps[core24].components.extra.revision",
            "presence is invalid"
        ));
        assert!(has(
            &with(json!({"extra": {"revision": "3"}}), false),
            "snaps[core24].components.extra.revision",
            "core24 has no revision"
        ));
        assert!(has(
            &with(json!({"extra": "sometimes"}), false),
            "snaps[core24].components.extra.presence",
            "required, optional or invalid"
        ));
        assert!(has(
            &with(json!(["extra"]), false),
            "snaps[core24].components",
            "map of components"
        ));
    }

    #[test]
    fn channels() {
        assert_eq!(channel_has_track("stable"), Ok(false));
        assert_eq!(channel_has_track("stable/fix"), Ok(false));
        assert_eq!(channel_has_track("24"), Ok(true));
        assert_eq!(channel_has_track("24/stable"), Ok(true));
        assert_eq!(channel_has_track("latest/edge/fix"), Ok(true));
        assert!(channel_has_track("24/bogus").is_err());
        assert!(channel_has_track("a//b").is_err());
    }

    #[test]
    fn times() {
        assert_eq!(rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(rfc3339("1970-01-01T01:00:00+01:00"), Some(0));
        assert_eq!(rfc3339("2024-04-19T08:42:32+00:00"), Some(1713516152));
        assert_eq!(
            rfc3339("2026-11-27T10:00:00.123Z"),
            rfc3339("2026-11-27T10:00:00Z")
        );
        assert_eq!(rfc3339("2026-11-27 10:00:00Z"), None);
        assert_eq!(rfc3339("2026-13-27T10:00:00Z"), None);
        assert_eq!(
            year_later("2024-02-29T00:00:00Z"),
            rfc3339("2025-03-01T00:00:00Z")
        );
    }

    #[test]
    fn names() {
        assert!(valid_snap_name("pc-kernel"));
        assert!(!valid_snap_name("pc--kernel"));
        assert!(!valid_snap_name("123"));
        assert!(!valid_snap_name("a"));
        assert!(valid_username("dev.ops_1"));
        assert!(!valid_username("-dev"));
    }
}
