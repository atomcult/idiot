//! Redacting snapd's secrets from a parsed state.json.
//!
//! Every `snapd-state` mode calls [`redact`] right after parsing, so no
//! output can carry an unredacted value. The rules (the inspect spec's
//! "Redacting snapd's secrets"):
//!
//! - known paths: under `data.auth` the users' `macaroon`, `discharges`,
//!   `store-macaroon` and `store-discharges`, the device's
//!   `session-macaroon` and `macaroon-key`; `data.snap-cookies` and
//!   `data.refresh-privacy-key` (`overlord/auth/auth.go`,
//!   `overlord/snapstate/cookies.go`, `overlord/snapstate/snapmgr.go`);
//! - anywhere, a member whose name (ignoring case) contains one of
//!   [`SECRET_WORDS`], or is `key` or ends in `-key`/`_key`, except
//!   [`KEPT_KEYS`];
//! - anywhere, a string that is a serialised macaroon (`MDAx`, at least 64
//!   characters) becomes `<redacted>`, and a URL's `user:password@`
//!   becomes `<redacted>@`.
//!
//! Under a redacted member the shape survives: every string, number and
//! boolean becomes `<redacted>`, arrays keep their length, and object
//! member names become `<redacted-1>`, `<redacted-2>`… in order. The count
//! returned is the number of values replaced (each scalar under a
//! redacted member, each macaroon string, each URL's credentials).

use serde_json::{Map, Value};

pub const REDACTED: &str = "<redacted>";

/// Member-name fragments that mark a value as secret.
const SECRET_WORDS: &[&str] = &[
    "macaroon",
    "discharge",
    "password",
    "passphrase",
    "secret",
    "token",
    "cookie",
    "credential",
    "private",
];

/// `-key` members that name things rather than unlock them.
const KEPT_KEYS: &[&str] = &["instance-key", "hotplug-key"];

/// Paths (member names from the root, `*` for any array index) that are
/// secret whatever the name rule says. Each is also caught by the name
/// rule today; they are listed so that a rule change can't uncover them.
const SECRET_PATHS: &[&[&str]] = &[
    &["data", "auth", "users", "*", "macaroon"],
    &["data", "auth", "users", "*", "discharges"],
    &["data", "auth", "users", "*", "store-macaroon"],
    &["data", "auth", "users", "*", "store-discharges"],
    &["data", "auth", "device", "session-macaroon"],
    &["data", "auth", "macaroon-key"],
    &["data", "snap-cookies"],
    &["data", "refresh-privacy-key"],
];

/// Redact `state` in place; returns how many values were replaced.
pub fn redact(state: &mut Value) -> usize {
    let mut path = Vec::new();
    walk(state, &mut path)
}

fn walk(v: &mut Value, path: &mut Vec<String>) -> usize {
    match v {
        Value::String(s) => redact_string(s),
        Value::Array(a) => {
            let mut n = 0;
            for (i, item) in a.iter_mut().enumerate() {
                path.push(i.to_string());
                n += walk(item, path);
                path.pop();
            }
            n
        }
        Value::Object(o) => {
            let mut n = 0;
            for (name, item) in o.iter_mut() {
                path.push(name.clone());
                n += if secret_member(name) || secret_path(path) {
                    scrub(item)
                } else {
                    walk(item, path)
                };
                path.pop();
            }
            n
        }
        _ => 0,
    }
}

pub fn secret_member(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if KEPT_KEYS.contains(&lower.as_str()) {
        return false;
    }
    SECRET_WORDS.iter().any(|w| lower.contains(w))
        || lower == "key"
        || lower.ends_with("-key")
        || lower.ends_with("_key")
}

fn secret_path(path: &[String]) -> bool {
    SECRET_PATHS.iter().any(|p| {
        p.len() == path.len()
            && p.iter().zip(path).all(|(want, got)| {
                *want == got || (*want == "*" && got.bytes().all(|b| b.is_ascii_digit()))
            })
    })
}

/// Replace everything under a redacted member, keeping its shape.
fn scrub(v: &mut Value) -> usize {
    match v {
        Value::Null => 0,
        Value::String(_) | Value::Number(_) | Value::Bool(_) => {
            *v = Value::String(REDACTED.into());
            1
        }
        Value::Array(a) => a.iter_mut().map(scrub).sum(),
        Value::Object(o) => {
            let old = std::mem::take(o);
            let mut n = 0;
            let mut new = Map::new();
            for (i, (_, mut item)) in old.into_iter().enumerate() {
                n += scrub(&mut item);
                new.insert(format!("<redacted-{}>", i + 1), item);
            }
            *o = new;
            n
        }
    }
}

/// A serialised macaroon: base64 of "001…" (the first packet's length).
fn is_macaroon(s: &str) -> bool {
    s.starts_with("MDAx") && s.len() >= 64
}

fn redact_string(s: &mut String) -> usize {
    if is_macaroon(s) {
        *s = REDACTED.into();
        return 1;
    }
    let (out, n) = redact_url_userinfo(s);
    if n > 0 {
        *s = out;
    }
    n
}

/// Replace `user:password@` after every `scheme://` in `s`.
fn redact_url_userinfo(s: &str) -> (String, usize) {
    let mut out = String::with_capacity(s.len());
    let mut n = 0;
    let mut rest = s;
    while let Some(i) = rest.find("://") {
        let (before, after) = rest.split_at(i + 3);
        out.push_str(before);
        rest = after;
        if !has_scheme(&before[..i]) {
            continue;
        }
        // the authority ends at a path, query, fragment, space or quote
        let end = rest
            .find(|c: char| {
                matches!(c, '/' | '?' | '#' | '"' | '\'' | '<' | '>') || c.is_whitespace()
            })
            .unwrap_or(rest.len());
        let authority = &rest[..end];
        if let Some(at) = authority.rfind('@') {
            if authority[..at].contains(':') {
                out.push_str(REDACTED);
                rest = &rest[at..];
                n += 1;
            }
        }
    }
    out.push_str(rest);
    (out, n)
}

/// Whether `s` ends in a URL scheme: a letter, then letters, digits, `+`,
/// `-` or `.`.
fn has_scheme(s: &str) -> bool {
    let scheme: String = s
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        .collect();
    scheme
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names() {
        for n in [
            "macaroon",
            "Store-Macaroon",
            "discharges",
            "db.password",
            "PASSPHRASE",
            "client_secret",
            "auth-token",
            "snap-cookies",
            "credentials",
            "private",
            "key",
            "cohort-key",
            "refresh-privacy-key",
            "api_key",
        ] {
            assert!(secret_member(n), "{n} should be secret");
        }
        for n in [
            "instance-key",
            "Hotplug-Key",
            "key-id",
            "keys",
            "keyboard",
            "email",
        ] {
            assert!(!secret_member(n), "{n} should be kept");
        }
    }

    #[test]
    fn shape_survives() {
        let mut v = json!({"secret": {"a": [1, true, null, "x"], "b": {"c": 2}}});
        assert_eq!(redact(&mut v), 4);
        assert_eq!(
            v,
            json!({"secret": {"<redacted-1>": ["<redacted>", "<redacted>", null, "<redacted>"],
                              "<redacted-2>": {"<redacted-1>": "<redacted>"}}})
        );
    }

    #[test]
    fn urls() {
        let r = |s: &str| redact_url_userinfo(s);
        assert_eq!(
            r("http://bob:hunter2@proxy:3128"),
            ("http://<redacted>@proxy:3128".into(), 1)
        );
        assert_eq!(
            r("get https://a:b@x/p?u=c://d:e@f then ftp://g:h@i done"),
            (
                "get https://<redacted>@x/p?u=c://<redacted>@f then ftp://<redacted>@i done".into(),
                3
            )
        );
        assert_eq!(
            r("http://bob@proxy:3128"),
            ("http://bob@proxy:3128".into(), 0)
        );
        assert_eq!(
            r("http://proxy:3128/a@b"),
            ("http://proxy:3128/a@b".into(), 0)
        );
        assert_eq!(r("://a:b@c"), ("://a:b@c".into(), 0));
        assert_eq!(r("plain"), ("plain".into(), 0));
    }

    #[test]
    fn macaroon_values() {
        let long = format!("MDAx{}", "a".repeat(60));
        let mut v = json!({"log": [long, "MDAxshort"]});
        assert_eq!(redact(&mut v), 1);
        assert_eq!(v, json!({"log": ["<redacted>", "MDAxshort"]}));
    }
}
