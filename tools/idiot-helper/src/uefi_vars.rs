//! `uefi-vars`: list the variables in an OVMF/AAVMF variable store.
//!
//! Usage: idiot-helper uefi-vars <fw_vars.fd>
//!
//! `idiot vm vars` reads a VM's saved UEFI variables (the pflash file next
//! to its TPM state). No common tool reads that file: efi-readvar only
//! reads the running machine's variables, and virt-fw-vars
//! (python3-virt-firmware) is seldom installed. This reads the store
//! itself: every live variable with its GUID and size, and for the
//! secure-boot databases (PK, KEK, db, dbx) each signature list:
//! certificate subjects, hash counts. Output goes to stdout. A port of the
//! former share/uefi-vars.py with the same output and errors.
//!
//! Exit status: 0, 1 when the file isn't a variable store or can't be read.

use std::io::Write;

const AUTH_STORE: &str = "aaf32c78-947b-439a-a180-2e144ec37792";
const PLAIN_STORE: &str = "ddcf3616-3275-4164-98b6-fe85707ffe7d";
const CERT_X509: &str = "a5c059a1-94e4-4aa7-87b5-ab155c2bf072";
const SIG_TYPES: &[(&str, &str)] = &[
    (CERT_X509, "X.509"),
    ("c1c41626-504c-4092-aca9-41f936934328", "SHA-256"),
    ("3c5766e8-269c-4e34-aa14-ed776e85b3b6", "RSA-2048"),
    ("826ca512-cf10-4ac9-b187-be01496631bd", "SHA-1"),
];
const SECURE_BOOT_DBS: &[&str] = &["PK", "KEK", "db", "dbx", "dbt", "dbr"];
// a live variable: VAR_ADDED, possibly mid-update (VAR_IN_DELETED_TRANSITION)
const LIVE_STATES: &[u8] = &[0x3F, 0x3E];

/// `data[a:b]` with Python's clamping.
fn slice(d: &[u8], a: usize, b: usize) -> &[u8] {
    let a = a.min(d.len());
    &d[a..b.clamp(a, d.len())]
}

/// `uuid.UUID(bytes_le=raw)` as text.
fn guid(raw: &[u8]) -> Result<String, String> {
    if raw.len() != 16 {
        return Err("bytes_le is not a 16-char string".into());
    }
    let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
    let rev = |b: &[u8]| b.iter().rev().copied().collect::<Vec<u8>>();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        hex(&rev(&raw[0..4])),
        hex(&rev(&raw[4..6])),
        hex(&rev(&raw[6..8])),
        hex(&raw[8..10]),
        hex(&raw[10..16])
    ))
}

/// struct.unpack_from of a little-endian integer of `n` bytes, with
/// struct.error's message.
fn unpack(d: &[u8], off: usize, n: usize, total: usize) -> Result<u64, String> {
    if off + n > d.len() {
        return Err(format!(
            "unpack_from requires a buffer of at least {} bytes for unpacking {total} bytes at offset {off} (actual buffer size is {})",
            off + total,
            d.len()
        ));
    }
    Ok(d[off..off + n]
        .iter()
        .rev()
        .fold(0u64, |a, &b| (a << 8) | b as u64))
}

/// `raw.decode("utf-16-le")`, with UnicodeDecodeError's message.
fn utf16le(raw: &[u8]) -> Result<String, String> {
    let err = |s: usize, e: usize, why: &str| {
        if e - s == 1 {
            format!(
                "'utf-16-le' codec can't decode byte 0x{:02x} in position {s}: {why}",
                raw[s]
            )
        } else {
            format!(
                "'utf-16-le' codec can't decode bytes in position {s}-{}: {why}",
                e - 1
            )
        }
    };
    let mut out = String::new();
    let mut i = 0;
    while i < raw.len() {
        if i + 1 >= raw.len() {
            return Err(err(i, raw.len(), "truncated data"));
        }
        let u = u16::from_le_bytes([raw[i], raw[i + 1]]);
        if (0xDC00..=0xDFFF).contains(&u) {
            return Err(err(i, i + 2, "illegal encoding"));
        }
        if (0xD800..=0xDBFF).contains(&u) {
            if i + 3 >= raw.len() {
                return Err(err(i, raw.len(), "unexpected end of data"));
            }
            let lo = u16::from_le_bytes([raw[i + 2], raw[i + 3]]);
            if !(0xDC00..=0xDFFF).contains(&lo) {
                return Err(err(i, i + 2, "illegal UTF-16 surrogate"));
            }
            let c = 0x10000 + ((u32::from(u) - 0xD800) << 10) + (u32::from(lo) - 0xDC00);
            out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
            i += 4;
            continue;
        }
        out.push(char::from_u32(u32::from(u)).unwrap_or('\u{fffd}'));
        i += 2;
    }
    Ok(out)
}

struct Var {
    name: String,
    vendor: String,
    data: Vec<u8>,
}

/// Every live variable in the store.
fn variables(blob: &[u8]) -> Result<Vec<Var>, String> {
    if slice(blob, 40, 44) != b"_FVH" {
        return Err("not a UEFI firmware volume (no _FVH signature)".into());
    }
    let fvh = unpack(blob, 48, 2, 2)? as usize;
    let store = guid(slice(blob, fvh, fvh + 16))?;
    let (header, auth) = if store == AUTH_STORE {
        (60, true)
    } else if store == PLAIN_STORE {
        (32, false)
    } else {
        return Err(format!("unknown variable store {store}"));
    };
    let store_size = unpack(blob, fvh + 16, 4, 4)? as usize;
    let mut off = fvh + 28;
    let end = blob.len().min(fvh + store_size);
    let mut out = Vec::new();
    while off + header <= end {
        let start_id = unpack(blob, off, 2, 8)?;
        let state = unpack(blob, off + 2, 1, 8)? as u8;
        if start_id != 0x55AA {
            break;
        }
        let (name_size, data_size, vendor) = if auth {
            // StartId State Reserved Attributes MonotonicCount(8)
            // TimeStamp(16) PubKeyIndex(4), then NameSize DataSize VendorGuid
            (
                unpack(blob, off + 36, 4, 8)? as usize,
                unpack(blob, off + 40, 4, 8)? as usize,
                guid(slice(blob, off + 44, off + 60))?,
            )
        } else {
            (
                unpack(blob, off + 8, 4, 8)? as usize,
                unpack(blob, off + 12, 4, 8)? as usize,
                guid(slice(blob, off + 16, off + 32))?,
            )
        };
        let name_at = off + header;
        let data_at = name_at + name_size;
        if LIVE_STATES.contains(&state) {
            let name = utf16le(slice(blob, name_at, data_at))?;
            out.push(Var {
                name: name.trim_end_matches('\0').to_string(),
                vendor,
                data: slice(blob, data_at, data_at + data_size).to_vec(),
            });
        }
        off = (data_at + data_size + 3) & !3;
    }
    Ok(out)
}

/// (tag, content start, content end) of the DER item at off.
fn der(blob: &[u8], off: usize) -> Option<(u8, usize, usize)> {
    let tag = *blob.get(off)?;
    let mut length = *blob.get(off + 1)? as usize;
    let mut off = off + 2;
    if length & 0x80 != 0 {
        let count = length & 0x7F;
        length = slice(blob, off, off + count).iter().fold(0usize, |a, &b| {
            a.saturating_mul(256).saturating_add(b as usize)
        });
        off += count;
    }
    Some((tag, off, off.saturating_add(length)))
}

fn der_children(blob: &[u8], mut start: usize, end: usize) -> Option<Vec<(u8, usize, usize)>> {
    let mut out = Vec::new();
    while start < end {
        let item = der(blob, start)?;
        out.push(item);
        start = item.2;
    }
    Some(out)
}

/// The subject of a DER X.509 certificate as `CN=..., O=...`, or `?`.
fn cert_subject(cert: &[u8]) -> String {
    let subject = || -> Option<String> {
        let (_, cstart, _) = der(cert, 0)?; // Certificate
        let (_, tstart, tend) = der(cert, cstart)?; // tbsCertificate
        let mut fields = der_children(cert, tstart, tend)?;
        if fields.first()?.0 == 0xA0 {
            fields.remove(0); // [0] version
        }
        let (_, sstart, send) = *fields.get(4)?; // serial sigalg issuer validity SUBJECT
        let mut parts = Vec::new();
        for (_, rstart, rend) in der_children(cert, sstart, send)? {
            for (_, astart, aend) in der_children(cert, rstart, rend)? {
                let kids = der_children(cert, astart, aend)?;
                let (&(_, ostart, oend), &(_, vstart, vend)) = (kids.first()?, kids.get(1)?);
                let key = match slice(cert, ostart, oend) {
                    [0x55, 0x04, 0x03] => "CN",
                    [0x55, 0x04, 0x0a] => "O",
                    [0x55, 0x04, 0x06] => "C",
                    _ => continue,
                };
                parts.push(format!(
                    "{key}={}",
                    String::from_utf8_lossy(slice(cert, vstart, vend))
                ));
            }
        }
        Some(parts.join(", "))
    };
    match subject() {
        Some(s) if !s.is_empty() => s,
        _ => "?".into(),
    }
}

/// Describe each EFI_SIGNATURE_LIST in a secure-boot database.
fn signature_lists(data: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off + 28 <= data.len() {
        let Ok(sig_type) = guid(&data[off..off + 16]) else {
            break;
        };
        let le = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
        let (list_size, header_size, sig_size) = (
            le(off + 16) as usize,
            le(off + 20) as i64,
            le(off + 24) as i64,
        );
        if list_size < 28 || sig_size < 16 {
            break;
        }
        let kind = SIG_TYPES
            .iter()
            .find(|(g, _)| *g == sig_type)
            .map_or(sig_type.clone(), |(_, k)| k.to_string());
        let sigs_at = off as i64 + 28 + header_size;
        let count = (list_size as i64 - 28 - header_size).div_euclid(sig_size);
        if sig_type == CERT_X509 {
            for i in 0..count.max(0) {
                let at = (sigs_at + i * sig_size) as usize;
                let cert = slice(data, at + 16, at + sig_size as usize);
                out.push(format!("{kind}: {}", cert_subject(cert)));
            }
        } else {
            let s = if count == 1 { "y" } else { "ies" };
            out.push(format!("{kind}: {count} entr{s}"));
        }
        off += list_size;
    }
    out
}

pub fn run(args: &[String]) -> u8 {
    let [path] = args else {
        eprintln!("usage: uefi-vars.py <fw_vars.fd>");
        return 1;
    };
    let blob = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{path}: {}", crate::py::strerror(&e));
            return 1;
        }
    };
    let found = match variables(&blob) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{path}: {e}");
            return 1;
        }
    };
    let mut out = String::new();
    for v in &found {
        out += &format!("{}  ({})  {} bytes\n", v.name, v.vendor, v.data.len());
        if SECURE_BOOT_DBS.contains(&v.name.as_str()) {
            for line in signature_lists(&v.data) {
                out += &format!("    {line}\n");
            }
        }
    }
    if found.is_empty() {
        out += "(no variables)\n";
    }
    let _ = std::io::stdout().lock().write_all(out.as_bytes());
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid_le(s: &str) -> Vec<u8> {
        let h: Vec<u8> = (0..32)
            .step_by(2)
            .map(|i| {
                let hex: String = s.replace('-', "");
                u8::from_str_radix(&hex[i..i + 2], 16).unwrap()
            })
            .collect();
        let mut b = Vec::new();
        b.extend(h[0..4].iter().rev());
        b.extend(h[4..6].iter().rev());
        b.extend(h[6..8].iter().rev());
        b.extend(&h[8..16]);
        b
    }

    /// A plain (non-authenticated) store with one variable.
    fn store(name: &str, data: &[u8]) -> Vec<u8> {
        let mut b = vec![0u8; 72];
        b[40..44].copy_from_slice(b"_FVH");
        b[48..50].copy_from_slice(&72u16.to_le_bytes());
        b.extend(guid_le(PLAIN_STORE));
        b.extend(4096u32.to_le_bytes());
        b.extend([0u8; 8]);
        let name16: Vec<u8> = name
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        b.extend(0x55AAu16.to_le_bytes());
        b.push(0x3F);
        b.push(0);
        b.extend(7u32.to_le_bytes());
        b.extend((name16.len() as u32).to_le_bytes());
        b.extend((data.len() as u32).to_le_bytes());
        b.extend(guid_le("8be4df61-93ca-11d2-aa0d-00e098032b8c"));
        b.extend(name16);
        b.extend(data);
        b
    }

    fn siglist(kind: &str, sigs: &[Vec<u8>]) -> Vec<u8> {
        let size = 16 + sigs[0].len();
        let mut b = guid_le(kind);
        b.extend(((28 + size * sigs.len()) as u32).to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b.extend((size as u32).to_le_bytes());
        for s in sigs {
            b.extend([0u8; 16]);
            b.extend(s);
        }
        b
    }

    #[test]
    fn plain_store_lists() {
        let blob = store("BootOrder", &[1, 0]);
        let v = variables(&blob).unwrap();
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].name, "BootOrder");
        assert_eq!(v[0].vendor, "8be4df61-93ca-11d2-aa0d-00e098032b8c");
        assert_eq!(v[0].data, [1, 0]);
    }

    #[test]
    fn signature_lists_count_and_subjects() {
        // a minimal certificate: SEQ { SEQ { [0]v, serial, alg, issuer, validity,
        // subject { SET { SEQ { OID CN, "Test" } } } } }
        let subject = [
            0x30, 0x0d, 0x31, 0x0b, 0x30, 0x09, 0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x02, b'T',
            b'K',
        ];
        let mut tbs = vec![
            0xa0, 0x00, 0x02, 0x01, 0x01, 0x30, 0x00, 0x30, 0x00, 0x30, 0x00,
        ];
        tbs.extend(subject);
        let mut tbs_seq = vec![0x30, tbs.len() as u8];
        tbs_seq.extend(tbs);
        let mut cert = vec![0x30, tbs_seq.len() as u8];
        cert.extend(tbs_seq);
        let mut db = siglist(CERT_X509, &[cert]);
        db.extend(siglist(
            "c1c41626-504c-4092-aca9-41f936934328",
            &[vec![0; 32], vec![1; 32]],
        ));
        assert_eq!(signature_lists(&db), ["X.509: CN=TK", "SHA-256: 2 entries"]);
    }

    #[test]
    fn utf16_matches_python() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("testdata/py-vectors.json")).unwrap();
        for case in v["utf16"].as_array().unwrap() {
            let raw: Vec<u8> = case[0]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_u64().unwrap() as u8)
                .collect();
            match &case[1] {
                serde_json::Value::String(s) => {
                    assert_eq!(utf16le(&raw).as_deref(), Ok(s.as_str()))
                }
                e => assert_eq!(
                    utf16le(&raw),
                    Err(e["err"].as_str().unwrap().to_string()),
                    "{raw:?}"
                ),
            }
        }
    }

    #[test]
    fn not_a_store() {
        assert_eq!(
            variables(&[0u8; 100]).err().unwrap(),
            "not a UEFI firmware volume (no _FVH signature)"
        );
    }
}
