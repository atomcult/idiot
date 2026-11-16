//! Python behaviour the ported code's output depends on (Python 3.12, as
//! Ubuntu 24.04 ships it). Each is checked against vectors generated with
//! that Python (src/testdata/py-vectors.json).

/// `str.isspace()` for one character: Rust's White_Space plus the
/// information separators \x1c-\x1f, which Python counts as whitespace.
pub fn isspace(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `str.rstrip()`.
pub fn rstrip(s: &str) -> &str {
    s.trim_end_matches(isspace)
}

/// `str.strip()`.
pub fn strip(s: &str) -> &str {
    s.trim_matches(isspace)
}

fn bytes_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `bytes.strip()`.
pub fn bytes_strip(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| !bytes_space(c)).unwrap_or(b.len());
    let end = b
        .iter()
        .rposition(|&c| !bytes_space(c))
        .map_or(start, |e| e + 1);
    &b[start..end.max(start)]
}

/// `bytes.lstrip()`.
pub fn bytes_lstrip(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| !bytes_space(c)).unwrap_or(b.len());
    &b[start..]
}

/// `bytes.split()`: runs of ASCII whitespace separate the words.
pub fn bytes_split(b: &[u8]) -> Vec<&[u8]> {
    b.split(|&c| bytes_space(c))
        .filter(|w| !w.is_empty())
        .collect()
}

/// `data.replace(b"\r\n", b"\n")`.
pub fn replace_crlf(d: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(d.len());
    let mut i = 0;
    while i < d.len() {
        if d[i] == b'\r' && d.get(i + 1) == Some(&b'\n') {
            i += 1;
            continue;
        }
        out.push(d[i]);
        i += 1;
    }
    out
}

/// An OSError's `strerror`: the message without Rust's " (os error N)".
pub fn strerror(e: &std::io::Error) -> String {
    let s = e.to_string();
    match s.rfind(" (os error ") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}

/// `str(OSError)` for a file: `[Errno N] message: 'path'`.
pub fn os_error_str(e: &std::io::Error, path: &str) -> String {
    match e.raw_os_error() {
        Some(n) => format!("[Errno {n}] {}: {}", strerror(e), repr(path)),
        None => e.to_string(),
    }
}

/// Whether Python's `str.isprintable()` holds for c. Python asks Unicode's
/// tables; this covers the non-printable characters that matter here
/// (controls, separators other than space, format characters, private
/// use). An unassigned code point is printed as Python 3.12 would print an
/// assigned one.
fn printable(c: char) -> bool {
    let u = c as u32;
    !matches!(u,
        0x00..=0x1f | 0x7f..=0xa0 | 0xad
        | 0x600..=0x605 | 0x61c | 0x6dd | 0x70f | 0x890..=0x891 | 0x8e2 | 0x180e
        | 0x1680 | 0x2000..=0x200f | 0x2028..=0x202f | 0x205f..=0x2064 | 0x2066..=0x206f
        | 0x3000 | 0xd800..=0xf8ff | 0xfeff | 0xfff9..=0xfffb
        | 0x110bd | 0x110cd | 0x13430..=0x1343f | 0x1bca0..=0x1bca3 | 0x1d173..=0x1d17a
        | 0xe0001 | 0xe0020..=0xe007f | 0xf0000..=0x10ffff)
}

/// `repr(str)`.
pub fn repr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == q => {
                out.push('\\');
                out.push(c);
            }
            c if c == ' ' || (c.is_ascii() && !c.is_ascii_control()) => out.push(c),
            c if !c.is_ascii() && printable(c) => out.push(c),
            c if (c as u32) < 0x100 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c if (c as u32) < 0x10000 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push_str(&format!("\\U{:08x}", c as u32)),
        }
    }
    out.push(q);
    out
}

fn b64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// `base64.b64decode(data, validate=strict)`: binascii.a2b_base64. Lenient
/// mode skips characters outside the alphabet and stops at the padding
/// that completes a quad; strict mode rejects both. Errors carry Python's
/// messages.
pub fn b64decode(data: &[u8], strict: bool) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    let (mut quad, mut left, mut pads) = (0u8, 0u8, 0u8);
    let mut padding_started = false;
    if strict && data.first() == Some(&b'=') {
        return Err("Leading padding not allowed".into());
    }
    for (i, &ch) in data.iter().enumerate() {
        if ch == b'=' {
            padding_started = true;
            if strict && quad == 0 {
                return Err("Excess padding not allowed".into());
            }
            if quad >= 2 {
                pads += 1;
                if quad + pads >= 4 {
                    if strict && i + 1 < data.len() {
                        return Err("Excess data after padding".into());
                    }
                    return Ok(out);
                }
            }
            continue;
        }
        let Some(v) = b64_value(ch) else {
            if strict {
                return Err("Only base64 data is allowed".into());
            }
            continue;
        };
        if strict && padding_started {
            return Err("Discontinuous padding not allowed".into());
        }
        pads = 0;
        match quad {
            0 => {
                quad = 1;
                left = v;
            }
            1 => {
                quad = 2;
                out.push((left << 2) | (v >> 4));
                left = v & 0x0f;
            }
            2 => {
                quad = 3;
                out.push((left << 4) | (v >> 2));
                left = v & 0x03;
            }
            _ => {
                quad = 0;
                out.push((left << 6) | v);
                left = 0;
            }
        }
    }
    match quad {
        0 => Ok(out),
        1 => Err(format!(
            "Invalid base64-encoded string: number of data characters ({}) cannot be 1 more than a multiple of 4",
            out.len() / 3 * 4 + 1
        )),
        _ => Err("Incorrect padding".into()),
    }
}

fn digits(s: &str) -> Option<u32> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn month_days(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `HH[:MM[:SS[.ffffff]]]` or `HH[MM[SS[.ffffff]]]` as (h, m, s, µs).
fn parse_clock(t: &str) -> Option<(i64, i64, i64, i64)> {
    // Python 3.11+ takes ',' too, and any number of digits (truncated)
    let (main, frac) = match t.split_once(['.', ',']) {
        Some((a, f)) => (a, Some(f)),
        None => (t, None),
    };
    let parts: Vec<&str> = if main.contains(':') {
        main.split(':').collect()
    } else {
        main.as_bytes()
            .chunks(2)
            .map(|c| std::str::from_utf8(c).unwrap_or("x"))
            .collect()
    };
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|p| p.len() != 2) {
        return None;
    }
    let n: Vec<i64> = parts
        .iter()
        .map(|p| digits(p).map(i64::from))
        .collect::<Option<_>>()?;
    let (h, m, s) = (n[0], *n.get(1).unwrap_or(&0), *n.get(2).unwrap_or(&0));
    if h > 23 || m > 59 || s > 59 {
        return None;
    }
    let us = match frac {
        None => 0,
        Some(f) if parts.len() == 3 && !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()) => {
            let f = &f[..f.len().min(6)];
            i64::from(digits(f)?) * 10i64.pow(6 - f.len() as u32)
        }
        Some(_) => return None,
    };
    Some((h, m, s, us))
}

/// An ISO week date at the start of s (`2026-W44[-1]`, `2026W44[1]`): its
/// day number and length.
fn week_date(s: &str) -> Option<(i64, usize)> {
    let b = s.as_bytes();
    let (year, week, day, n) = if b.len() >= 8 && b[4] == b'-' && b[5] == b'W' {
        if b.len() >= 10 && b[8] == b'-' {
            (&s[0..4], &s[6..8], &s[9..10], 10)
        } else {
            (&s[0..4], &s[6..8], "1", 8)
        }
    } else if b.len() >= 7 && b[4] == b'W' {
        if b.len() >= 8 && b[7].is_ascii_digit() {
            (&s[0..4], &s[5..7], &s[7..8], 8)
        } else {
            (&s[0..4], &s[5..7], "1", 7)
        }
    } else {
        return None;
    };
    let (y, w, d) = (
        i64::from(digits(year)?),
        i64::from(digits(week)?),
        i64::from(digits(day)?),
    );
    let jan1 = days_from_civil(y, 1, 1);
    // Monday 0 .. Sunday 6 (1970-01-01 was a Thursday)
    let wd = |days: i64| (days + 3).rem_euclid(7);
    let long_year = wd(jan1) == 3 || (wd(jan1) == 2 && month_days(y, 2) == 29);
    if y < 1 || !(1..=7).contains(&d) || !(1..=53).contains(&w) || (w == 53 && !long_year) {
        return None;
    }
    let jan4 = days_from_civil(y, 1, 4);
    Some((jan4 - wd(jan4) + (w - 1) * 7 + (d - 1), n))
}

/// Python's `datetime.fromisoformat(s)` for the forms RFC 3339 times
/// take, as (seconds since the epoch in UTC, microseconds); None where
/// Python would raise. A time without an offset is UTC.
fn fromisoformat(s: &str) -> Option<(i64, i64)> {
    let b = s.as_bytes();
    let (days, rest) = if let Some((days, n)) = week_date(s) {
        (days, &s[n..])
    } else {
        let (date, rest) = if b.len() >= 10 && b[4] == b'-' && b[7] == b'-' {
            (
                (digits(&s[0..4])?, digits(&s[5..7])?, digits(&s[8..10])?),
                &s[10..],
            )
        } else if b.len() >= 8
            && s.is_char_boundary(8)
            && s[..8].bytes().all(|c| c.is_ascii_digit())
        {
            (
                (digits(&s[0..4])?, digits(&s[4..6])?, digits(&s[6..8])?),
                &s[8..],
            )
        } else {
            return None;
        };
        let (y, mo, d) = (i64::from(date.0), i64::from(date.1), i64::from(date.2));
        if y < 1 || !(1..=12).contains(&mo) || d < 1 || d > month_days(y, mo) {
            return None;
        }
        (days_from_civil(y, mo, d), rest)
    };
    let mut secs = days * 86400;
    let mut us = 0;
    // any one character separates the date and the time
    let mut chars = rest.chars();
    if chars.next().is_some() {
        let time = chars.as_str();
        let (mut clock, zone) = match time.find(['+', '-']) {
            Some(i) => (&time[..i], Some(&time[i..])),
            None => (time, None),
        };
        // CPython's parser skips one stray character between a time
        // without a fraction and its offset ("10:05:00 +00:00")
        if zone.is_some() && !clock.contains(['.', ',']) {
            if let Some(c) = clock.chars().last().filter(|c| !c.is_ascii_digit()) {
                clock = &clock[..clock.len() - c.len_utf8()];
            }
        }
        let (h, m, sec, micro) = parse_clock(clock)?;
        secs += h * 3600 + m * 60 + sec;
        us = micro;
        if let Some(z) = zone {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            let (zh, zm, zs, zus) = parse_clock(&z[1..])?;
            if zus != 0 {
                return None;
            }
            secs -= sign * (zh * 3600 + zm * 60 + zs);
        }
    }
    Some((secs, us))
}

/// norm_time from assertion.py: the comparable form of an RFC 3339 time,
/// as UTC `datetime.isoformat()`, or the cleaned text when it isn't one.
pub fn norm_time(t: &str) -> String {
    let mut s = strip(t).replace('Z', "+00:00");
    if let Some((head, tail)) = s.split_once('.') {
        // datetime wants at most 6 fractional digits
        let frac: String = tail.chars().filter(char::is_ascii_digit).collect();
        let zone: String = tail.chars().skip(frac.chars().count()).collect();
        s = format!("{head}.{}{zone}", &format!("{frac}000000")[..6]);
    }
    let Some((secs, us)) = fromisoformat(&s) else {
        return s;
    };
    let (y, m, d) = civil_from_days(secs.div_euclid(86400));
    let rem = secs.rem_euclid(86400);
    let frac = if us != 0 {
        format!(".{us:06}")
    } else {
        String::new()
    };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}{frac}+00:00",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("testdata/py-vectors.json")).unwrap()
    }

    fn bytes(v: &Value) -> Vec<u8> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|b| b.as_u64().unwrap() as u8)
            .collect()
    }

    #[test]
    fn repr_matches_python() {
        for v in vectors()["repr"].as_array().unwrap() {
            assert_eq!(repr(v[0].as_str().unwrap()), v[1].as_str().unwrap());
        }
    }

    #[test]
    fn b64decode_matches_python() {
        for v in vectors()["b64"].as_array().unwrap() {
            let input = bytes(&v[0]);
            let strict = v[1].as_bool().unwrap();
            let got = b64decode(&input, strict);
            match &v[2] {
                Value::String(err) if strict => assert!(got.is_err(), "{input:?} {err}"),
                Value::String(err) => assert_eq!(got, Err(err.clone()), "{input:?}"),
                ok => assert_eq!(got, Ok(bytes(ok)), "{input:?} strict={strict}"),
            }
        }
    }

    #[test]
    fn norm_time_matches_python() {
        for v in vectors()["norm_time"].as_array().unwrap() {
            assert_eq!(
                norm_time(v[0].as_str().unwrap()),
                v[1].as_str().unwrap(),
                "{}",
                v[0]
            );
        }
    }

    #[test]
    fn rstrip_matches_python() {
        for v in vectors()["rstrip"].as_array().unwrap() {
            assert_eq!(rstrip(v[0].as_str().unwrap()), v[1].as_str().unwrap());
        }
    }
}
