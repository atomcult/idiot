//! `assertion`: summarise and verify a stream of snapd assertions.
//!
//! A port of the former share/assertion.py, kept output-for-output: the same
//! arguments, report, --facts file and exit status, so cmd/assertion/show
//! only changed the line that runs it.
//!
//! Usage: idiot-helper assertion [--remote] [--no-verify] [--snap <path>]
//!            [--trusted <file>]... [--facts <file>] [--name <label>] <file|->
//!
//! `idiot assertion show` reads model, account-key, serial, system-user,
//! snap-declaration and other assertions without root and without touching
//! snapd's assertion database (`snap ack` needs both). The parser follows
//! snapd's text format, and a signature is checked with the RSA arithmetic
//! of PKCS#1 v1.5 over SHA-512, comparing the whole padded block. The
//! account-keys a signature needs come from the stream itself, then this
//! machine's database (`snap known`, with --snap), then the --trusted
//! bundles, then (with --remote) the store (`snap known --remote`). A key
//! from the database, or a self-signed key from a bundle, ends a chain:
//! those are the keys snapd trusts.
//!
//! The report goes to stdout, colored with the OUT_* values in the
//! environment. --facts writes one line per model for the shell's
//! cross-checks, fields separated by 0x1f (so an empty one isn't lost to the
//! shell's field splitting): index, brand-id, model, authority-id, store,
//! revision, and a file holding the model's own text.
//!
//! Exit status: 0, 1 when any assertion is invalid, 2 on unreadable or
//! unparsable input.
//!
//! Where Python's behaviour shows in the output it is reproduced: `%r` in
//! parse errors (`py::repr`), base64's lenient and strict decoding
//! (`py::b64decode`), the time normalisation the key validity checks
//! compare (`py::norm_time`), and padding by characters.

use crate::py;
use base64::Engine;
use num_bigint::BigUint;
use sha2::{Digest, Sha512};
use sha3::Sha3_384;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ------------------------------------------------------------------ values

/// A header value: text, a list or a map (in the order written, like the
/// Python dict it replaces).
#[derive(Clone, Debug, PartialEq)]
pub enum V {
    S(String),
    L(Vec<V>),
    M(Vec<(String, V)>),
}

impl V {
    fn as_str(&self) -> Option<&str> {
        match self {
            V::S(s) => Some(s),
            _ => None,
        }
    }
    fn get(&self, k: &str) -> Option<&V> {
        match self {
            V::M(m) => m.iter().find(|(key, _)| key == k).map(|(_, v)| v),
            _ => None,
        }
    }
    fn truthy(&self) -> bool {
        match self {
            V::S(s) => !s.is_empty(),
            V::L(l) => !l.is_empty(),
            V::M(m) => !m.is_empty(),
        }
    }
    /// Python's repr of the value (str, list or dict).
    fn repr(&self) -> String {
        match self {
            V::S(s) => py::repr(s),
            V::L(l) => format!("[{}]", l.iter().map(V::repr).collect::<Vec<_>>().join(", ")),
            V::M(m) => format!(
                "{{{}}}",
                m.iter()
                    .map(|(k, v)| format!("{}: {}", py::repr(k), v.repr()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
    /// Python's str() of the value: `%s` formatting.
    fn py_str(&self) -> String {
        match self {
            V::S(s) => s.clone(),
            other => other.repr(),
        }
    }
}

/// An ordered header map: a new value for an existing key keeps its place.
type Map = Vec<(String, V)>;

fn insert(m: &mut Map, k: String, v: V) {
    match m.iter_mut().find(|(key, _)| *key == k) {
        Some(e) => e.1 = v,
        None => m.push((k, v)),
    }
}

fn get<'a>(m: &'a Map, k: &str) -> Option<&'a V> {
    m.iter().find(|(key, _)| key == k).map(|(_, v)| v)
}

/// h.get(k, default) as a string, for `%s`.
fn get_str(m: &Map, k: &str, default: &str) -> String {
    get(m, k)
        .map(V::py_str)
        .unwrap_or_else(|| default.to_string())
}

/// A str-typed header, or None.
fn get_text<'a>(m: &'a Map, k: &str) -> Option<&'a str> {
    get(m, k).and_then(V::as_str)
}

// ----------------------------------------------------------------- parsing

#[derive(Debug)]
pub struct ParseError(pub String);

/// `s[a:]` by characters.
fn from_char(s: &str, a: usize) -> &str {
    match s.char_indices().nth(a) {
        Some((i, _)) => &s[i..],
        None => "",
    }
}

fn head60(s: &str) -> String {
    py::repr(&s.chars().take(60).collect::<String>())
}

fn parse_value(lines: &[&str], mut i: usize, indent: usize) -> Result<(V, usize), ParseError> {
    let pre = " ".repeat(indent);
    let first = from_char(lines[i], indent);
    if first == "-" || first.starts_with("- ") {
        let mut items = Vec::new();
        while i < lines.len() && lines[i].starts_with(&pre) && {
            let r = from_char(lines[i], indent);
            r == "-" || r.starts_with("- ")
        } {
            let rest = from_char(from_char(lines[i], indent), 2);
            if !rest.is_empty() {
                items.push(V::S(rest.to_string()));
                i += 1;
            } else {
                i += 1;
                if i < lines.len() && lines[i].starts_with(&format!("{pre}  ")) {
                    let (v, n) = parse_value(lines, i, indent + 2)?;
                    items.push(v);
                    i = n;
                } else {
                    return Err(ParseError("empty list item".into()));
                }
            }
        }
        return Ok((V::L(items), i));
    }
    if first.contains(':') && !first.starts_with(' ') {
        let mut m = Map::new();
        while i < lines.len()
            && lines[i].starts_with(&pre)
            && !from_char(lines[i], indent).starts_with(' ')
        {
            i = parse_entry(lines, i, indent, &mut m)?;
        }
        return Ok((V::M(m), i));
    }
    let mut text = Vec::new();
    while i < lines.len() && lines[i].starts_with(&pre) {
        text.push(from_char(lines[i], indent));
        i += 1;
    }
    Ok((V::S(text.join("\n")), i))
}

fn parse_entry(
    lines: &[&str],
    i: usize,
    indent: usize,
    into: &mut Map,
) -> Result<usize, ParseError> {
    let line = from_char(lines[i], indent);
    let bad = || ParseError(format!("bad header line: {}", head60(lines[i])));
    let Some((k, v)) = line.split_once(':') else {
        return Err(bad());
    };
    if k.is_empty() || k.contains(' ') {
        return Err(bad());
    }
    if let Some(v) = v.strip_prefix(' ') {
        insert(into, k.to_string(), V::S(v.to_string()));
        return Ok(i + 1);
    }
    if !v.is_empty() {
        return Err(bad());
    }
    let i = i + 1;
    if i >= lines.len() || !lines[i].starts_with(&" ".repeat(indent + 2)) {
        return Err(ParseError(format!("header {k} has no value")));
    }
    let (v, n) = parse_value(lines, i, indent + 2)?;
    insert(into, k.to_string(), v);
    Ok(n)
}

fn parse_headers(text: &str) -> Result<Map, ParseError> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut h = Map::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].starts_with(' ') {
            return Err(ParseError(format!(
                "unexpected indented line: {}",
                head60(lines[i])
            )));
        }
        i = parse_entry(&lines, i, 0, &mut h)?;
    }
    Ok(h)
}

#[derive(Clone, Debug)]
pub struct Assertion {
    pub index: usize,
    pub h: Map,
    pub body: Vec<u8>,
    pub content: Vec<u8>,
    pub sig: Vec<u8>,
    pub raw: Vec<u8>,
}

impl Assertion {
    fn type_(&self) -> String {
        get_text(&self.h, "type").unwrap_or("").to_string()
    }
}

fn find(data: &[u8], pat: &[u8], from: usize) -> Option<usize> {
    if from > data.len() {
        return None;
    }
    data[from..]
        .windows(pat.len())
        .position(|w| w == pat)
        .map(|p| p + from)
}

/// `data[a:b]` with Python's clamping.
fn slice(data: &[u8], a: usize, b: usize) -> &[u8] {
    let a = a.min(data.len());
    let b = b.clamp(a, data.len());
    &data[a..b]
}

pub fn decode_stream(data: &[u8]) -> Result<Vec<Assertion>, ParseError> {
    let mut out: Vec<Assertion> = Vec::new();
    let n = data.len();
    let mut pos = 0;
    loop {
        while pos < n && matches!(data[pos], b'\n' | b'\r' | b' ' | b'\t') {
            pos += 1;
        }
        if pos >= n {
            break;
        }
        let idx = out.len() + 1;
        let where_ = format!("assertion {idx} (at byte {pos})");
        let err = |m: &str| ParseError(format!("{where_}: {m}"));
        let Some(end) = find(data, b"\n\n", pos) else {
            return Err(err("no signature (truncated?)"));
        };
        let htext =
            std::str::from_utf8(&data[pos..end]).map_err(|_| err("headers aren't UTF-8 text"))?;
        let h = parse_headers(htext).map_err(|e| err(&e.0))?;
        if get(&h, "type").is_none() {
            return Err(err("no type header"));
        }
        let blen = match get(&h, "body-length") {
            None => Some(0),
            Some(V::S(s)) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
                // a length past usize can't be satisfied: "shorter" below
                Some(s.parse::<usize>().unwrap_or(usize::MAX))
            }
            _ => None,
        };
        let Some(blen) = blen else {
            return Err(err("bad body-length"));
        };
        let (body, content, sstart);
        if blen > 0 {
            let b_end = (end + 2).saturating_add(blen);
            body = slice(data, end + 2, b_end).to_vec();
            if body.len() < blen || slice(data, b_end, b_end.saturating_add(2)) != b"\n\n" {
                return Err(err("body shorter than its body-length (truncated?)"));
            }
            content = data[pos..b_end].to_vec();
            sstart = b_end + 2;
        } else {
            body = Vec::new();
            content = data[pos..end].to_vec();
            sstart = end + 2;
        }
        let send = find(data, b"\n\n", sstart).unwrap_or(n);
        let sig = py::bytes_strip(slice(data, sstart, send));
        if sig.is_empty() {
            return Err(err("no signature (truncated?)"));
        }
        let joined: Vec<u8> = py::bytes_split(sig).concat();
        let sigbytes =
            py::b64decode(&joined, true).map_err(|_| err("the signature isn't base64"))?;
        let mut raw = content.clone();
        raw.extend_from_slice(b"\n\n");
        raw.extend_from_slice(sig);
        raw.push(b'\n');
        out.push(Assertion {
            index: idx,
            h,
            body,
            content,
            sig: sigbytes,
            raw,
        });
        pos = send;
    }
    Ok(out)
}

// --------------------------------------------------------- OpenPGP and RSA

const INDEX: &str = "index out of range";

fn at(d: &[u8], i: usize) -> Result<u8, String> {
    d.get(i).copied().ok_or_else(|| INDEX.to_string())
}

fn be(d: &[u8]) -> usize {
    d.iter().fold(0usize, |a, &b| (a << 8) | b as usize)
}

/// The tag and body of the OpenPGP packet at the start of data.
fn packet(data: &[u8]) -> Result<(u8, &[u8]), String> {
    if data.is_empty() {
        return Err("empty packet".into());
    }
    let tag = data[0];
    if tag & 0x80 == 0 {
        return Err("not an OpenPGP packet".into());
    }
    if tag & 0x40 != 0 {
        let ln = at(data, 1)? as usize;
        if ln < 192 {
            return Ok((tag & 0x3F, slice(data, 2, 2 + ln)));
        }
        if ln < 224 {
            let ln = ((ln - 192) << 8) + at(data, 2)? as usize + 192;
            return Ok((tag & 0x3F, slice(data, 3, 3 + ln)));
        }
        if ln == 255 {
            let ln = be(slice(data, 2, 6));
            return Ok((tag & 0x3F, slice(data, 6, 6usize.saturating_add(ln))));
        }
        return Err("partial-length packet".into());
    }
    let lt = tag & 3;
    if lt == 3 {
        return Err("indeterminate-length packet".into());
    }
    let w = [1usize, 2, 4][lt as usize];
    let ln = be(slice(data, 1, 1 + w));
    Ok((
        (tag >> 2) & 0x0F,
        slice(data, 1 + w, (1 + w).saturating_add(ln)),
    ))
}

fn mpi(d: &[u8], o: usize) -> Result<(BigUint, usize), String> {
    let bits = (at(d, o)? as usize) << 8 | at(d, o + 1)? as usize;
    let ln = bits.div_ceil(8);
    if o + 2 + ln > d.len() {
        return Err("truncated MPI".into());
    }
    Ok((BigUint::from_bytes_be(&d[o + 2..o + 2 + ln]), o + 2 + ln))
}

fn b64sha3(raw: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha3_384::digest(raw))
}

/// An account-key's public key.
struct Key {
    label: String,
    digest: String,
    n: Option<BigUint>,
    e: Option<BigUint>,
    algo: String,
}

impl Key {
    fn new(a: &Assertion) -> Result<Key, String> {
        let raw = py::b64decode(&py::bytes_split(&a.body).concat(), false)?;
        let digest = b64sha3(&raw);
        let label = format!(
            "{}/{}",
            get_str(&a.h, "account-id", "?"),
            get_str(&a.h, "name", "?")
        );
        if raw.first() != Some(&1) {
            return Err("unknown public key format".into());
        }
        let (tag, p) = packet(&raw[1..])?;
        if tag != 6 || at(p, 0)? != 4 {
            return Err("not an OpenPGP v4 public key".into());
        }
        let mut k = Key {
            label,
            digest,
            n: None,
            e: None,
            algo: "unknown".into(),
        };
        let alg = at(p, 5)?;
        if matches!(alg, 1..=3) {
            k.algo = "RSA".into();
            let (n, o) = mpi(p, 6)?;
            let (e, _) = mpi(p, o)?;
            k.n = Some(n);
            k.e = Some(e);
        } else {
            k.algo = format!("algorithm {alg}");
        }
        Ok(k)
    }

    fn bits(&self) -> u64 {
        self.n.as_ref().map_or(0, |n| n.bits())
    }

    fn has_n(&self) -> bool {
        self.n.as_ref().is_some_and(|n| n.bits() > 0)
    }
}

// DER DigestInfo prefix for SHA-512 (RFC 8017, 9.2)
const SHA512_INFO: [u8; 19] = [
    0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03, 0x05,
    0x00, 0x04, 0x40,
];

type Verdict = (&'static str, String);

/// None when a's signature verifies with key, else (kind, reason).
fn check_signature(a: &Assertion, key: &Key) -> Option<Verdict> {
    let s = &a.sig;
    if s.first() != Some(&1) {
        return Some(("invalid", "unknown signature format".into()));
    }
    let parsed = (|| -> Result<Result<(Vec<u8>, BigUint), Verdict>, String> {
        let (tag, p) = packet(&s[1..])?;
        if tag != 2 || at(p, 0)? != 4 {
            return Ok(Err(("invalid", "not an OpenPGP v4 signature".into())));
        }
        let (pk, ha) = (at(p, 2)?, at(p, 3)?);
        let hl = (at(p, 4)? as usize) << 8 | at(p, 5)? as usize;
        let hashed = slice(p, 0, 6 + hl).to_vec();
        let ul = (at(p, 6 + hl)? as usize) << 8 | at(p, 7 + hl)? as usize;
        let o = 8 + hl + ul + 2;
        if pk != 1 && pk != 3 {
            return Ok(Err((
                "unverified",
                format!("unsupported signature algorithm {pk}"),
            )));
        }
        if ha != 10 {
            return Ok(Err((
                "unverified",
                format!("unsupported hash algorithm {ha}"),
            )));
        }
        if key.algo != "RSA" {
            return Ok(Err((
                "unverified",
                format!("unsupported key algorithm ({})", key.algo),
            )));
        }
        let (sv, _) = mpi(p, o)?;
        Ok(Ok((hashed, sv)))
    })();
    let (hashed, sv) = match parsed {
        Err(_) => return Some(("invalid", "malformed signature".into())),
        Ok(Err(v)) => return Some(v),
        Ok(Ok(x)) => x,
    };
    let (n, e) = (key.n.as_ref()?, key.e.as_ref()?);
    let mut h = Sha512::new();
    h.update(&a.content);
    h.update(&hashed);
    h.update([0x04, 0xff]);
    h.update((hashed.len() as u32).to_be_bytes());
    let digest = h.finalize();
    let k = n.bits().div_ceil(8) as usize;
    let tlen = SHA512_INFO.len() + digest.len();
    let no_match = Some(("invalid", "signature doesn't match".to_string()));
    if &sv >= n || k < tlen + 11 {
        return no_match;
    }
    let mut expected = vec![0x00, 0x01];
    expected.extend(std::iter::repeat_n(0xff, k - tlen - 3));
    expected.push(0x00);
    expected.extend_from_slice(&SHA512_INFO);
    expected.extend_from_slice(&digest);
    let got = sv.modpow(e, n).to_bytes_be();
    let mut padded = vec![0u8; k.saturating_sub(got.len())];
    padded.extend(got);
    if padded != expected {
        return no_match;
    }
    None
}

// ------------------------------------------------------------ verification

#[derive(Clone, Copy, PartialEq)]
enum Source {
    Stream,
    Bundle,
    Local,
    Remote,
}

type Found = (usize, Source, bool);

struct Verifier {
    snap: Option<String>,
    remote: bool,
    /// every account-key seen, by index into `keys`
    keys: Vec<Assertion>,
    stream: HashMap<String, usize>,
    bundle: HashMap<String, usize>,
    cache: HashMap<String, Option<Found>>,
}

impl Verifier {
    fn new(stream: &[Assertion], snap: Option<String>, trusted: &[String], remote: bool) -> Self {
        let mut v = Verifier {
            snap,
            remote,
            keys: Vec::new(),
            stream: HashMap::new(),
            bundle: HashMap::new(),
            cache: HashMap::new(),
        };
        let stream_keys: Vec<Assertion> = stream.iter().map(clone_assertion).collect();
        v.stream = v.add_keys(stream_keys);
        for f in trusted {
            match std::fs::read(f)
                .map_err(|e| py::os_error_str(&e, f))
                .and_then(|d| decode_stream(&d).map_err(|e| e.0))
            {
                Ok(list) => {
                    let m = v.add_keys(list);
                    v.bundle.extend(m);
                }
                Err(e) => eprintln!("idiot: warning: can't read {f}: {e}"),
            }
        }
        v
    }

    /// Keep the account-keys of `list`, by their public-key-sha3-384.
    fn add_keys(&mut self, list: Vec<Assertion>) -> HashMap<String, usize> {
        let mut m = HashMap::new();
        for a in list {
            if a.type_() != "account-key" {
                continue;
            }
            let Some(d) = get(&a.h, "public-key-sha3-384").filter(|v| v.truthy()) else {
                continue;
            };
            let d = d.py_str();
            self.keys.push(a);
            m.insert(d, self.keys.len() - 1);
        }
        m
    }

    fn known(&mut self, digest: &str, remote: bool) -> Option<usize> {
        let snap = self.snap.clone()?;
        let mut cmd = Command::new(snap);
        cmd.arg("known");
        if remote {
            cmd.arg("--remote");
        }
        cmd.args(["account-key", &format!("public-key-sha3-384={digest}")]);
        let out = run_timeout(cmd, Duration::from_secs(60))?;
        let list = decode_stream(&out).ok()?;
        let m = self.add_keys(list);
        m.get(digest).copied()
    }

    /// The account-key for digest: (its index, where from, trusted). A key
    /// snapd trusts (in the local database, or self-signed in a bundle) is
    /// returned as that trusted copy even when the stream carries it too,
    /// so a stream can't relabel a trusted key.
    fn find(&mut self, digest: &str) -> Option<Found> {
        if let Some(f) = self.cache.get(digest) {
            return *f;
        }
        let local = self.known(digest, false);
        let anchor = self.bundle.get(digest).copied();
        let found = if let Some(l) = local {
            Some((l, Source::Local, true))
        } else if let Some(an) =
            anchor.filter(|&i| get_text(&self.keys[i].h, "sign-key-sha3-384") == Some(digest))
        {
            Some((an, Source::Bundle, true))
        } else {
            let mut f = self
                .stream
                .get(digest)
                .map(|&i| (i, Source::Stream))
                .or(anchor.map(|i| (i, Source::Bundle)));
            if f.is_none() && self.remote {
                f = self.known(digest, true).map(|i| (i, Source::Remote));
            }
            f.map(|(i, s)| (i, s, false))
        };
        self.cache.insert(digest.to_string(), found);
        found
    }

    fn verify(&mut self, a: &Assertion, depth: usize) -> Verdict {
        let kd = match get(&a.h, "sign-key-sha3-384") {
            Some(V::S(s)) if !s.is_empty() => s.clone(),
            _ => return ("invalid", "no sign-key-sha3-384 header".into()),
        };
        if depth > 4 {
            return (
                "unverified",
                "the chain of account-keys is longer than 4".into(),
            );
        }
        let Some((ki, _source, trusted)) = self.find(&kd) else {
            let hint = if self.snap.is_some() && !self.remote {
                " (try --remote)"
            } else {
                ""
            };
            return ("unverified", format!("account-key {kd} not found{hint}"));
        };
        let ka = clone_assertion(&self.keys[ki]);
        let key = match Key::new(&ka) {
            Ok(k) => k,
            Err(e) => return ("invalid", format!("account-key {kd}: {e}")),
        };
        if key.digest != kd {
            return (
                "invalid",
                format!("account-key {kd} doesn't hash to its public-key-sha3-384"),
            );
        }
        let auth = get(&a.h, "authority-id")
            .cloned()
            .unwrap_or(V::S(String::new()));
        if get(&ka.h, "account-id") != Some(&auth) {
            let shown = if auth.truthy() {
                auth.py_str()
            } else {
                "(none)".into()
            };
            return (
                "invalid",
                format!(
                    "signed with a key of {}, not of its authority {shown}",
                    get_str(&ka.h, "account-id", "?")
                ),
            );
        }
        let when = [get(&a.h, "timestamp"), get(&a.h, "since")]
            .into_iter()
            .flatten()
            .find(|v| v.truthy());
        let since = get_str(&ka.h, "since", "");
        let until = get_str(&ka.h, "until", "");
        if let Some(V::S(when)) = when {
            let t = py::norm_time(when);
            if !since.is_empty() && t < py::norm_time(&since) {
                return (
                    "invalid",
                    format!("signed at {when}, before the key is valid (since {since})"),
                );
            }
            if !until.is_empty() && t >= py::norm_time(&until) {
                return (
                    "invalid",
                    format!("signed at {when}, after the key expired (until {until})"),
                );
            }
        }
        if let Some(bad) = check_signature(a, &key) {
            return bad;
        }
        if trusted {
            return ("verified", key.label);
        }
        if get_text(&ka.h, "sign-key-sha3-384") == Some(kd.as_str()) {
            return (
                "unverified",
                format!(
                    "the chain ends at {}, a self-signed key nothing trusts",
                    key.label
                ),
            );
        }
        let (kind, text) = self.verify(&ka, depth + 1);
        if kind == "invalid" {
            return (kind, format!("account-key {}: {text}", key.label));
        }
        (kind, text)
    }
}

fn clone_assertion(a: &Assertion) -> Assertion {
    Assertion {
        index: a.index,
        h: a.h.clone(),
        body: a.body.clone(),
        content: a.content.clone(),
        sig: a.sig.clone(),
        raw: a.raw.clone(),
    }
}

/// Run cmd with stdout captured and stderr discarded; None if it can't run
/// or outlives the timeout (then it's killed). The exit status doesn't
/// matter: what it printed is used.
fn run_timeout(mut cmd: Command, timeout: Duration) -> Option<Vec<u8>> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < timeout => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    reader.join().ok()
}

// ---------------------------------------------------------------- printing

struct Colors(HashMap<&'static str, String>);

impl Colors {
    fn from_env() -> Self {
        let mut m = HashMap::new();
        for k in ["BOLD", "DIM", "RED", "GREEN", "YELLOW", "CYAN", "RESET"] {
            m.insert(k, std::env::var(format!("OUT_{k}")).unwrap_or_default());
        }
        Colors(m)
    }
    fn c(&self, k: &str) -> &str {
        &self.0[k]
    }
}

fn width(s: &str) -> usize {
    s.chars().count()
}

fn entries(v: &V) -> usize {
    match v {
        V::L(l) => l.len(),
        V::M(m) => m.len(),
        V::S(_) => 1,
    }
}

fn table(rows: &[Vec<String>], c: &Colors) -> Vec<String> {
    if rows.is_empty() {
        return Vec::new();
    }
    let cols = rows[0].len();
    let w: Vec<usize> = (0..cols)
        .map(|i| rows.iter().map(|r| width(&r[i])).max().unwrap_or(0))
        .collect();
    rows.iter()
        .map(|r| {
            let cells: Vec<String> = r
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let pad = if i < r.len() - 1 {
                        format!("{cell:<w$}", w = w[i])
                    } else {
                        cell.clone()
                    };
                    if i == 0 {
                        format!("{}{pad}{}", c.c("CYAN"), c.c("RESET"))
                    } else if i == r.len() - 1 {
                        format!("{}{pad}{}", c.c("DIM"), c.c("RESET"))
                    } else {
                        pad
                    }
                })
                .collect();
            format!("  {}", py::rstrip(&cells.join("  ")))
        })
        .collect()
}

fn sget(s: &V, k: &str, default: &str) -> String {
    s.get(k).map(V::py_str).unwrap_or_else(|| default.into())
}

const PRIMARY: &[(&str, &[&str])] = &[
    ("account", &["account-id"]),
    ("account-key", &["public-key-sha3-384"]),
    ("base-declaration", &["series"]),
    ("confdb-schema", &["account-id", "name"]),
    ("model", &["series", "brand-id", "model"]),
    ("preseed", &["series", "brand-id", "model", "system-label"]),
    ("repair", &["brand-id", "repair-id"]),
    ("serial", &["brand-id", "model", "serial"]),
    ("snap-build", &["snap-sha3-384"]),
    ("snap-declaration", &["series", "snap-id"]),
    ("snap-developer", &["snap-id", "publisher-id"]),
    (
        "snap-resource-pair",
        &[
            "snap-id",
            "resource-name",
            "resource-revision",
            "snap-revision",
        ],
    ),
    (
        "snap-resource-revision",
        &["snap-id", "resource-name", "resource-sha3-384"],
    ),
    ("snap-revision", &["snap-sha3-384"]),
    ("store", &["store"]),
    ("system-user", &["brand-id", "email"]),
    (
        "validation",
        &[
            "series",
            "snap-id",
            "approved-snap-id",
            "approved-snap-revision",
        ],
    ),
    (
        "validation-set",
        &["series", "account-id", "name", "sequence"],
    ),
];
// headers shown first, after the primary key, when present
const LEADING: &[&str] = &[
    "authority-id",
    "brand-id",
    "account-id",
    "name",
    "sign-key-sha3-384",
    "timestamp",
    "revision",
];
// headers never printed as such
const HIDDEN: &[&str] = &["type", "body-length", "password"];

fn validity(h: &Map) -> String {
    format!(
        "{} → {}",
        get_str(h, "since", "?"),
        get_str(h, "until", "(no end)")
    )
}

/// (label, value) lines and extra lines for assertion a.
fn summary(a: &Assertion, c: &Colors) -> (Vec<(String, String)>, Vec<String>) {
    let h = &a.h;
    let t = a.type_();
    let mut shown: HashSet<String> = HIDDEN.iter().map(|s| s.to_string()).collect();
    let mut lines = vec![(
        "type".to_string(),
        format!("{}{t}{}", c.c("BOLD"), c.c("RESET")),
    )];
    let primary = PRIMARY
        .iter()
        .find(|(k, _)| *k == t)
        .map(|(_, v)| *v)
        .unwrap_or(&[]);
    for k in primary.iter().chain(LEADING) {
        if let Some(V::S(v)) = get(h, k) {
            if shown.insert(k.to_string()) {
                lines.push((k.to_string(), v.clone()));
            }
        }
    }
    let mut extra = Vec::new();
    let dicts = |v: &V| -> Vec<V> {
        match v {
            V::L(l) => l.iter().filter(|x| matches!(x, V::M(_))).cloned().collect(),
            _ => Vec::new(),
        }
    };
    match t.as_str() {
        "model" => {
            if let Some(snaps @ V::L(_)) = get(h, "snaps") {
                shown.insert("snaps".into());
                let rows: Vec<Vec<String>> = dicts(snaps)
                    .iter()
                    .map(|s| {
                        vec![
                            sget(s, "name", "?"),
                            sget(s, "type", "app"),
                            sget(s, "default-channel", ""),
                            sget(s, "presence", "required"),
                            sget(s, "id", ""),
                        ]
                    })
                    .collect();
                extra.push("snaps:".into());
                extra.extend(table(&rows, c));
            }
            if let Some(vs @ V::L(_)) = get(h, "validation-sets") {
                shown.insert("validation-sets".into());
                let brand = get_str(h, "brand-id", "?");
                let rows: Vec<Vec<String>> = dicts(vs)
                    .iter()
                    .map(|s| {
                        let name =
                            format!("{}/{}", sget(s, "account-id", &brand), sget(s, "name", "?"));
                        let seq = sget(s, "sequence", "");
                        vec![
                            name,
                            sget(s, "mode", ""),
                            if seq.is_empty() {
                                String::new()
                            } else {
                                format!("sequence {seq}")
                            },
                        ]
                    })
                    .collect();
                extra.push("validation-sets:".into());
                extra.extend(table(&rows, c));
            }
            if let Some(V::L(rs)) = get(h, "required-snaps") {
                shown.insert("required-snaps".into());
                let names: Vec<&str> = rs.iter().filter_map(V::as_str).collect();
                extra.push(format!("required-snaps: {}", names.join(", ")));
            }
        }
        "account-key" => {
            match Key::new(a) {
                Ok(key) => {
                    let info = if key.has_n() {
                        format!("{} {} bits", key.algo, key.bits())
                    } else {
                        key.algo.clone()
                    };
                    let matches = get(h, "public-key-sha3-384") == Some(&V::S(key.digest.clone()));
                    let note = if matches {
                        String::new()
                    } else {
                        format!(
                            "  {}(doesn't match public-key-sha3-384){}",
                            c.c("RED"),
                            c.c("RESET")
                        )
                    };
                    lines.push(("public key".into(), info));
                    lines.push(("key sha3-384".into(), format!("{}{note}", key.digest)));
                }
                Err(e) => lines.push((
                    "public key".into(),
                    format!("{}unreadable: {e}{}", c.c("RED"), c.c("RESET")),
                )),
            }
            lines.push(("valid".into(), validity(h)));
            shown.insert("since".into());
            shown.insert("until".into());
        }
        "validation-set" => {
            if let Some(snaps @ V::L(_)) = get(h, "snaps") {
                shown.insert("snaps".into());
                let rows: Vec<Vec<String>> = dicts(snaps)
                    .iter()
                    .map(|s| {
                        let rev = sget(s, "revision", "");
                        vec![
                            sget(s, "name", "?"),
                            sget(s, "presence", "required"),
                            if rev.is_empty() {
                                "any revision".into()
                            } else {
                                format!("revision {rev}")
                            },
                            sget(s, "id", ""),
                        ]
                    })
                    .collect();
                extra.push("snaps:".into());
                extra.extend(table(&rows, c));
            }
        }
        "system-user" => {
            if let Some(V::L(models)) = get(h, "models") {
                shown.insert("models".into());
                let m: Vec<&str> = models.iter().filter_map(V::as_str).collect();
                lines.push(("models".into(), m.join(", ")));
            }
            lines.push(("valid".into(), validity(h)));
            shown.insert("since".into());
            shown.insert("until".into());
            let keys = match get(h, "ssh-keys") {
                Some(V::L(l)) => l.len(),
                _ => 0,
            };
            shown.insert("ssh-keys".into());
            lines.push(("ssh-keys".into(), keys.to_string()));
            let set = get(h, "password").is_some_and(V::truthy);
            lines.push((
                "password".into(),
                if set { "set" } else { "not set" }.into(),
            ));
        }
        _ => {}
    }
    for (k, v) in h {
        if shown.contains(k) {
            continue;
        }
        match v {
            V::S(s) if !s.contains('\n') => lines.push((k.clone(), s.clone())),
            _ => {
                let n = entries(v);
                lines.push((
                    k.clone(),
                    format!("({n} {})", if n == 1 { "entry" } else { "entries" }),
                ));
            }
        }
    }
    if !a.body.is_empty() && t != "account-key" {
        lines.push(("body".into(), format!("{} bytes", a.body.len())));
    }
    (lines, extra)
}

fn die(msg: &str) -> u8 {
    eprintln!("idiot: error: {msg}");
    2
}

fn one_line(v: Option<&V>, default: &str) -> String {
    match v {
        None => default.replace(['\x1f', '\n'], " "),
        Some(V::S(s)) => s.replace(['\x1f', '\n'], " "),
        Some(_) => String::new(),
    }
}

pub fn run(argv: &[String]) -> u8 {
    let usage = "usage: assertion.py [options] <file|->";
    let (mut remote, mut no_verify) = (false, false);
    let (mut snap, mut facts, mut label): (Option<String>, Option<String>, Option<String>) =
        (None, None, None);
    let mut trusted = Vec::new();
    let mut path: Option<String> = None;
    let mut args = argv.iter();
    while let Some(x) = args.next() {
        match x.as_str() {
            "--remote" => remote = true,
            "--no-verify" => no_verify = true,
            "--snap" | "--trusted" | "--facts" | "--name" => {
                let Some(v) = args.next() else {
                    return die(&format!("{x} needs a value"));
                };
                match x.as_str() {
                    "--snap" => snap = Some(v.clone()),
                    "--trusted" => trusted.push(v.clone()),
                    "--name" => label = Some(v.clone()),
                    _ => facts = Some(v.clone()),
                }
            }
            _ if path.is_none() => path = Some(x.clone()),
            _ => return die(usage),
        }
    }
    let Some(path) = path else {
        return die(usage);
    };
    // `label or ...`: an empty --name counts as none
    let name = match label.filter(|l| !l.is_empty()) {
        Some(l) => l,
        None if path == "-" => "stdin".into(),
        None => path.clone(),
    };
    let snap = snap.filter(|s| !s.is_empty());
    let facts = facts.filter(|f| !f.is_empty());
    let data = if path == "-" {
        let mut d = Vec::new();
        std::io::stdin().read_to_end(&mut d).map(|_| d)
    } else {
        std::fs::read(&path)
    };
    let data = match data {
        Ok(d) => py::replace_crlf(&d),
        Err(e) => return die(&format!("can't read {name}: {}", py::strerror(&e))),
    };
    let stream = match decode_stream(&data) {
        Ok(s) => s,
        Err(e) => {
            if !py::bytes_lstrip(&data).starts_with(b"type:") {
                return die(&format!("no assertion found in {name}"));
            }
            return die(&format!("{name}: {}", e.0));
        }
    };
    if stream.is_empty() {
        return die(&format!("no assertion found in {name}"));
    }

    let c = Colors::from_env();
    let mut verifier = if no_verify {
        None
    } else {
        Some(Verifier::new(&stream, snap, &trusted, remote))
    };
    let mut status = 0;
    let mut fact_lines = Vec::new();
    let mut out = String::new();
    for a in &stream {
        if a.index > 1 {
            out.push('\n');
        }
        let (lines, extra) = summary(a, &c);
        let mut sigline = None;
        if let Some(v) = verifier.as_mut() {
            let (kind, text) = v.verify(a, 0);
            let col = match kind {
                "verified" => c.c("GREEN"),
                "invalid" => c.c("RED"),
                _ => c.c("YELLOW"),
            };
            let tail = if kind == "verified" {
                format!(" ({text})")
            } else {
                format!(": {text}")
            };
            sigline = Some(format!("{col}{kind}{}{tail}", c.c("RESET")));
            if kind == "invalid" {
                status = 1;
            }
        }
        let mut w = lines.iter().map(|(k, _)| width(k)).max().unwrap_or(0);
        if sigline.is_some() {
            w = w.max(width("signature"));
        }
        for (k, v) in &lines {
            out += &format!(
                "{}{k}:{} {}{v}\n",
                c.c("BOLD"),
                c.c("RESET"),
                " ".repeat(w - width(k))
            );
        }
        for x in &extra {
            out += x;
            out.push('\n');
        }
        if let Some(s) = &sigline {
            out += &format!(
                "{}signature:{} {}{s}\n",
                c.c("BOLD"),
                c.c("RESET"),
                " ".repeat(w - width("signature"))
            );
        }
        if a.type_() == "model" {
            if let Some(f) = &facts {
                let copy = format!("{f}.{}", a.index);
                if let Err(e) = std::fs::write(&copy, &a.raw) {
                    return die(&format!("can't write {copy}: {}", py::strerror(&e)));
                }
                let mut fields = vec![a.index.to_string()];
                for k in ["brand-id", "model", "authority-id", "store"] {
                    fields.push(one_line(get(&a.h, k), ""));
                }
                fields.push(one_line(get(&a.h, "revision"), "0"));
                fields.push(copy);
                fact_lines.push(fields.join("\x1f"));
            }
        }
    }
    if let Some(f) = &facts {
        let text: String = fact_lines.iter().map(|l| format!("{l}\n")).collect();
        if let Err(e) = std::fs::write(f, text) {
            return die(&format!("can't write {f}: {}", py::strerror(&e)));
        }
    }
    // a closed pipe (`| head`) isn't an error, as BrokenPipeError wasn't
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(out.as_bytes())
        .and_then(|_| stdout.flush())
        .is_err()
    {
        return 0;
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_keep_order_and_nesting() {
        let h = parse_headers(
            "type: model\nsnaps:\n  -\n    name: pc\n    type: gadget\n  -\n    name: core24\nlist:\n  - a\n  - b\ntext:\n  line one\n  line two\ntype: model2",
        )
        .unwrap();
        let keys: Vec<&str> = h.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["type", "snaps", "list", "text"]);
        assert_eq!(get_text(&h, "type"), Some("model2"));
        assert_eq!(get_text(&h, "text"), Some("line one\nline two"));
        let V::L(snaps) = get(&h, "snaps").unwrap() else {
            panic!()
        };
        assert_eq!(snaps[0].get("type"), Some(&V::S("gadget".into())));
    }

    #[test]
    fn parse_errors_quote_like_python() {
        let e = parse_headers("type: x\nbad line").unwrap_err();
        assert_eq!(e.0, "bad header line: 'bad line'");
        let e = parse_headers("type: x\n it's").unwrap_err();
        assert_eq!(e.0, "unexpected indented line: \" it's\"");
        let e = parse_headers("type: x\nsnaps:").unwrap_err();
        assert_eq!(e.0, "header snaps has no value");
    }

    #[test]
    fn stream_errors() {
        let e = decode_stream(b"type: x\nbody-length: 5\n\nab\n\nsig").unwrap_err();
        assert_eq!(
            e.0,
            "assertion 1 (at byte 0): body shorter than its body-length (truncated?)"
        );
        let e = decode_stream(b"type: x\n\n!!!!").unwrap_err();
        assert_eq!(e.0, "assertion 1 (at byte 0): the signature isn't base64");
        let e = decode_stream(b"type: x").unwrap_err();
        assert_eq!(e.0, "assertion 1 (at byte 0): no signature (truncated?)");
    }

    fn bundle() -> Vec<Assertion> {
        let data = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../share/assertions/canonical.account-keys"
        ))
        .unwrap();
        decode_stream(&data).unwrap()
    }

    fn verify_with_bundle(a: &Assertion) -> Verdict {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../share/assertions/canonical.account-keys"
        );
        let mut v = Verifier::new(&[], None, &[path.to_string()], false);
        v.verify(a, 0)
    }

    #[test]
    fn bundled_chain_verifies() {
        for a in bundle() {
            let (kind, text) = verify_with_bundle(&a);
            assert_eq!(kind, "verified", "{text}");
        }
    }

    #[test]
    fn flipped_bit_fails() {
        let mut a = bundle().pop().unwrap();
        let last = a.sig.len() - 1;
        a.sig[last] ^= 1;
        assert_eq!(
            verify_with_bundle(&a),
            ("invalid", "signature doesn't match".to_string())
        );
    }

    #[test]
    fn changed_content_fails() {
        let mut a = bundle().pop().unwrap();
        a.content.push(b' ');
        assert_eq!(verify_with_bundle(&a).0, "invalid");
    }

    #[test]
    fn wrong_key_fails() {
        // the root key's assertion, checked against the models key
        let keys = bundle();
        let root = keys
            .iter()
            .find(|a| get_text(&a.h, "name") == Some("root"))
            .unwrap();
        let other = keys
            .iter()
            .find(|a| get_text(&a.h, "name") != Some("root"))
            .unwrap();
        let key = Key::new(other).unwrap();
        assert_eq!(
            check_signature(root, &key),
            Some(("invalid", "signature doesn't match".to_string()))
        );
    }
}
