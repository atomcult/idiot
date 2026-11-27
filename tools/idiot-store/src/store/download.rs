//! Downloading a released snap revision for an architecture, verified
//! against the store's size and SHA3-384, into a cache directory, with the
//! assertions snapd needs to install it asserted.
//!
//! The store's download URL gets the credential (refreshed once when the
//! store asks); the CDN it redirects to never does: its URL carries its own
//! token. The body is streamed into a private `.partial` file while it is
//! hashed and renamed into place only when it matches.

use super::assertion::{self, Fetcher};
use super::query::{self, Auth};
use super::{credential, USER_AGENT};
use crate::error::StoreError;
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use sha3::{Digest, Sha3_384};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const CHUNK: usize = 64 * 1024;

/// Whether the snap came from the network or the cache.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum How {
    Downloaded,
    Cached,
}

/// A verified snap in the directory (and its `.assert` beside it, unless
/// assertions were skipped).
#[derive(Debug, PartialEq)]
pub struct Fetched {
    pub path: PathBuf,
    pub name: String,
    pub revision: u64,
    pub version: String,
    pub how: How,
}

/// What a download came to; the failures sh phrases are outcomes, not
/// errors.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Snap(Fetched),
    /// The store doesn't show the snap.
    Unknown,
    /// No release in `channel` for `arch`; `available` lists the channels
    /// that have one for it, with their revisions.
    Unreleased {
        channel: String,
        arch: String,
        available: Vec<(String, u64)>,
    },
    /// The download URL (or the CDN) refused, with this status.
    Refused(u16),
}

/// What one download is for: the snap, where it comes from and goes to.
pub struct Request<'a> {
    pub snap: &'a str,
    pub store: Option<&'a str>,
    pub arch: &'a str,
    pub channel: &'a str,
    pub dir: &'a Path,
    pub assertions: bool,
    pub progress: bool,
}

/// Resolves the release, downloads it (or reuses a cached copy that still
/// matches), and fetches its assertions.
pub fn download(mut auth: Option<&mut Auth>, rq: &Request) -> Result<Outcome, StoreError> {
    let c = query::client()?;
    let reply = query::send(&c, auth.as_deref_mut(), || {
        query::info_request(&c, rq.snap, rq.store)
    })?;
    let info = match reply.status {
        StatusCode::NOT_FOUND => return Ok(Outcome::Unknown),
        s if s.is_success() => reply.json(),
        _ => return Err(query::rejected_or(&reply)),
    };
    let snap_id = query::text(&info, "snap-id");
    let channel = query::normalise_channel(rq.channel);
    let map = query::parse_info(&info);
    let Some(entry) = map.iter().find(|e| e.name == channel && e.arch == rq.arch) else {
        return Ok(Outcome::Unreleased {
            channel,
            arch: rq.arch.to_string(),
            available: map
                .iter()
                .filter(|e| e.arch == rq.arch)
                .map(|e| (e.name.clone(), e.revision))
                .collect(),
        });
    };
    let name = match query::text(&info, "name") {
        n if n.is_empty() => rq.snap.to_string(),
        n => n,
    };
    let want = Expected {
        name: &name,
        revision: entry.revision,
        size: entry.size,
        sha3_384: &entry.sha3_384,
    };
    private_dir(rq.dir)?;
    clean_stale_partials(rq.dir);
    let (path, how) = match fetch_snap(auth.as_deref_mut(), &entry.url, rq.dir, &want, rq.progress)?
    {
        Ok(got) => got,
        Err(status) => return Ok(Outcome::Refused(status)),
    };
    if rq.assertions {
        let mut fetcher =
            Fetcher::new(auth, rq.store.map(str::to_owned), Some(rq.arch.to_string()))?;
        let path_key = format!(
            "snap-revision/{}",
            assertion::sha3_key(&want.sha3_384.to_ascii_lowercase())
        );
        let got = assertion::fetch(&mut fetcher, &path_key, true)?;
        check_revision(&got, &snap_id, &want)?;
        write_private(
            &rq.dir.join(format!("{name}_{}.assert", want.revision)),
            assertion::stream(&got).as_bytes(),
        )?;
    }
    Ok(Outcome::Snap(Fetched {
        path,
        name,
        revision: entry.revision,
        version: entry.version.clone(),
        how,
    }))
}

/// The store's description of the revision a file must match.
pub struct Expected<'a> {
    pub name: &'a str,
    pub revision: u64,
    pub size: u64,
    /// Hex, as the info response has it.
    pub sha3_384: &'a str,
}

impl Expected<'_> {
    fn file_name(&self) -> String {
        format!("{}_{}.snap", self.name, self.revision)
    }

    fn mismatch(&self, what: &str) -> StoreError {
        StoreError::Store(format!(
            "{what} mismatch for {} revision {}",
            self.name, self.revision
        ))
    }
}

/// Creates `dir` (mode 0700) when it is missing.
fn private_dir(dir: &Path) -> Result<(), StoreError> {
    if dir.is_dir() {
        return Ok(());
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| local(dir, e))
}

fn local(path: &Path, e: std::io::Error) -> StoreError {
    StoreError::Store(format!("{}: {e}", path.display()))
}

/// Removes `.partial` files left by downloads whose process is gone (an
/// interrupt kills the process before it can).
fn clean_stale_partials(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(rest) = name.strip_suffix(".partial") else {
            continue;
        };
        let Some(pid) = rest.rsplit('.').next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        if !Path::new(&format!("/proc/{pid}")).exists() {
            let _ = fs::remove_file(e.path());
        }
    }
}

fn partial_path(dir: &Path, file: &str) -> PathBuf {
    dir.join(format!(".{file}.{}.partial", std::process::id()))
}

/// Writes `data` to `path` (mode 0600) through a partial file and a rename.
fn write_private(path: &Path, data: &[u8]) -> Result<(), StoreError> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let file = path.file_name().unwrap_or_default().to_string_lossy();
    let partial = partial_path(dir, &file);
    let result = (|| {
        let mut f = create_private(&partial)?;
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&partial, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&partial);
        local(path, e)
    })
}

fn create_private(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
}

/// Whether the file at `path` has the expected size and digest.
pub fn matches(path: &Path, want: &Expected) -> bool {
    let Ok(mut f) = File::open(path) else {
        return false;
    };
    if f.metadata().map(|m| m.len()).ok() != Some(want.size) {
        return false;
    }
    let mut hasher = Sha3_384::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        match f.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return false,
        }
    }
    hex(&hasher.finalize()) == want.sha3_384.to_ascii_lowercase()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The snap in `dir`: the cached file when it still matches, else a fresh
/// download from `url` (Err: the status it was refused with).
pub fn fetch_snap(
    auth: Option<&mut Auth>,
    url: &str,
    dir: &Path,
    want: &Expected,
    progress: bool,
) -> Result<Result<(PathBuf, How), u16>, StoreError> {
    let path = dir.join(want.file_name());
    if path.exists() {
        if matches(&path, want) {
            return Ok(Ok((path, How::Cached)));
        }
        fs::remove_file(&path).map_err(|e| local(&path, e))?;
    }
    let resp = match get(auth, url)? {
        Ok(r) => r,
        Err(status) => return Ok(Err(status)),
    };
    let partial = partial_path(dir, &want.file_name());
    let result = stream_into(resp, &partial, want, progress)
        .and_then(|()| fs::rename(&partial, &path).map_err(|e| local(&path, e)));
    if let Err(e) = result {
        let _ = fs::remove_file(&partial);
        return Err(e);
    }
    Ok(Ok((path, How::Downloaded)))
}

/// A client for the body: no overall timeout (a kernel snap takes a while),
/// but connecting and each read may not stall. The blocking client's
/// `timeout` bounds each wait (the response, then every read of the body),
/// not the whole transfer.
fn body_client() -> Result<Client, StoreError> {
    Ok(Client::builder()
        .user_agent(USER_AGENT)
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(60))
        .build()?)
}

/// The response with the snap's body: the store's URL with the credential
/// (one refresh when asked), then by hand to the redirect's `Location`
/// without it. Err: the status a refusal came with.
fn get(auth: Option<&mut Auth>, url: &str) -> Result<Result<Response, u16>, StoreError> {
    let c = body_client()?;
    let attempt = |cred: Option<&str>| -> Result<Response, StoreError> {
        let mut rq = c.get(url);
        if let Some(cr) = cred {
            rq = rq.header("Authorization", credential::auth_header(cr)?);
        }
        Ok(rq.send()?)
    };
    let mut resp = match auth {
        None => attempt(None)?,
        Some(a) => {
            let first = attempt(Some(&a.credential))?;
            if first.status() != StatusCode::UNAUTHORIZED {
                first
            } else {
                // a 401's body is a short error, never the snap
                let r = query::reply(first);
                if !r.wants_refresh {
                    return Ok(Err(r.status.as_u16()));
                }
                a.credential = query::refresh(&query::client()?, &a.credential)?;
                attempt(Some(&a.credential))?
            }
        }
    };
    if resp.status().is_redirection() {
        let location = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| StoreError::Store("a redirect without a Location".into()))?;
        let target = resp
            .url()
            .join(location)
            .map_err(|e| StoreError::Store(format!("bad redirect {location}: {e}")))?;
        // a fresh request: the CDN's URL carries its own token
        resp = c.get(target).send()?;
    }
    match resp.status() {
        s if s.is_success() => Ok(Ok(resp)),
        s @ (StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND) => {
            Ok(Err(s.as_u16()))
        }
        s => Err(StoreError::Store(format!("download answered {s}"))),
    }
}

/// Copies the body into `partial` (mode 0600), hashing it, and checks the
/// size and digest; draws a progress line on stderr when asked.
fn stream_into(
    mut resp: Response,
    partial: &Path,
    want: &Expected,
    progress: bool,
) -> Result<(), StoreError> {
    let mut f = create_private(partial).map_err(|e| local(partial, e))?;
    let mut hasher = Sha3_384::new();
    let mut buf = vec![0u8; CHUNK];
    let mut got: u64 = 0;
    let mut drawn = Instant::now();
    let mb = |n: u64| n as f64 / 1_000_000.0;
    loop {
        let n = resp
            .read(&mut buf)
            .map_err(|e| StoreError::Network(e.to_string()))?;
        if n == 0 {
            break;
        }
        got += n as u64;
        if got > want.size {
            return Err(want.mismatch("size"));
        }
        hasher.update(&buf[..n]);
        f.write_all(&buf[..n]).map_err(|e| local(partial, e))?;
        if progress && drawn.elapsed() > Duration::from_millis(200) {
            eprint!(
                "\r{} {}: {:.1}/{:.1} MB",
                want.name,
                want.revision,
                mb(got),
                mb(want.size)
            );
            drawn = Instant::now();
        }
    }
    if progress {
        eprintln!(
            "\r{} {}: {:.1}/{:.1} MB",
            want.name,
            want.revision,
            mb(got),
            mb(want.size)
        );
    }
    f.sync_all().map_err(|e| local(partial, e))?;
    if got != want.size {
        return Err(want.mismatch("size"));
    }
    if hex(&hasher.finalize()) != want.sha3_384.to_ascii_lowercase() {
        return Err(want.mismatch("sha3-384"));
    }
    Ok(())
}

/// Checks the snap-revision in a fetched chain describes the revision the
/// info entry named: its snap-id, revision and size.
pub fn check_revision(chain: &[String], snap_id: &str, want: &Expected) -> Result<(), StoreError> {
    let rev = chain
        .iter()
        .map(|a| assertion::headers(a).0)
        .find(|h| h.iter().any(|(k, v)| k == "type" && v == "snap-revision"))
        .ok_or_else(|| StoreError::Store("no snap-revision in the assertions".into()))?;
    let get = |k: &str| {
        rev.iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    let revision = want.revision.to_string();
    let size = want.size.to_string();
    for (k, expected) in [
        ("snap-id", snap_id),
        ("snap-revision", revision.as_str()),
        ("snap-size", size.as_str()),
    ] {
        if get(k) != expected {
            return Err(StoreError::Store(format!(
                "the store's snap-revision for {} revision {} has {k} {}, not {expected}",
                want.name,
                want.revision,
                get(k)
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::assertion::Source;
    use crate::store::credential::tests::{macaroon, ser};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("idiot-store-dl-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn sha(data: &[u8]) -> String {
        hex(&Sha3_384::digest(data))
    }

    fn want<'a>(digest: &'a str, size: usize) -> Expected<'a> {
        Expected {
            name: "hello",
            revision: 7,
            size: size as u64,
            sha3_384: digest,
        }
    }

    /// Reads one request's head; true when it carries an Authorization header.
    fn read_head(s: &mut std::net::TcpStream) -> bool {
        let mut buf = Vec::new();
        let mut b = [0u8; 1];
        while !buf.ends_with(b"\r\n\r\n") {
            if s.read(&mut b).unwrap() == 0 {
                break;
            }
            buf.push(b[0]);
        }
        String::from_utf8_lossy(&buf)
            .to_ascii_lowercase()
            .contains("\r\nauthorization:")
    }

    fn body_response(body: &[u8]) -> Vec<u8> {
        let mut r = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes();
        r.extend_from_slice(body);
        r
    }

    fn credential() -> String {
        let root = macaroon("dashboard.snapcraft.io", "root");
        let discharge = macaroon("login.ubuntu.com", "discharge");
        credential::encode(&ser(&root), &ser(&discharge)).unwrap()
    }

    /// The store URL gets the credential and redirects; the second request,
    /// to the redirect's target, must carry none.
    #[test]
    fn redirect_without_credential() {
        let body = b"snap body".to_vec();
        let digest = sha(&body);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let served = body.clone();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            assert!(read_head(&mut s), "the store request lacks the credential");
            s.write_all(format!("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{port}/cdn?token=t\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).unwrap();
            let (mut s, _) = listener.accept().unwrap();
            if read_head(&mut s) {
                s.write_all(b"HTTP/1.1 500 Credential Leaked\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .unwrap();
                return false;
            }
            s.write_all(&body_response(&served)).unwrap();
            true
        });
        let dir = scratch("redirect");
        private_dir(&dir).unwrap();
        let mut auth = Auth::new(&credential());
        let got = fetch_snap(
            Some(&mut auth),
            &format!("http://127.0.0.1:{port}/download"),
            &dir,
            &want(&digest, body.len()),
            false,
        )
        .unwrap()
        .unwrap();
        assert!(
            server.join().unwrap(),
            "the CDN request carried the credential"
        );
        assert_eq!(got.1, How::Downloaded);
        assert_eq!(fs::read(&got.0).unwrap(), body);
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&got.0).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    fn serve_once(body: Vec<u8>) -> (u16, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            read_head(&mut s);
            s.write_all(&body_response(&body)).unwrap();
        });
        (port, h)
    }

    #[test]
    fn wrong_digest_keeps_nothing() {
        let (port, h) = serve_once(b"tampered".to_vec());
        let dir = scratch("digest");
        private_dir(&dir).unwrap();
        let good = sha(b"original");
        let e = fetch_snap(
            None,
            &format!("http://127.0.0.1:{port}/x"),
            &dir,
            &want(&good, 8),
            false,
        )
        .unwrap_err();
        h.join().unwrap();
        assert_eq!(e.exit_code(), 3);
        assert!(e.to_string().contains("sha3-384 mismatch"), "{e}");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn good_cache_needs_no_request() {
        let dir = scratch("cached");
        private_dir(&dir).unwrap();
        let body = b"cached body";
        let digest = sha(body);
        let w = want(&digest, body.len());
        fs::write(dir.join(w.file_name()), body).unwrap();
        // nothing listens on port 9: a request would fail
        let got = fetch_snap(None, "http://127.0.0.1:9/never", &dir, &w, false)
            .unwrap()
            .unwrap();
        assert_eq!(got.1, How::Cached);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn bad_cache_downloaded_again() {
        let body = b"the real snap".to_vec();
        let digest = sha(&body);
        let (port, h) = serve_once(body.clone());
        let dir = scratch("badcache");
        private_dir(&dir).unwrap();
        let w = want(&digest, body.len());
        fs::write(dir.join(w.file_name()), b"the real sna").unwrap();
        let got = fetch_snap(None, &format!("http://127.0.0.1:{port}/x"), &dir, &w, false)
            .unwrap()
            .unwrap();
        h.join().unwrap();
        assert_eq!(got.1, How::Downloaded);
        assert_eq!(fs::read(&got.0).unwrap(), body);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn refusal_is_an_outcome() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            read_head(&mut s);
            s.write_all(
                b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        });
        let dir = scratch("refused");
        private_dir(&dir).unwrap();
        let got = fetch_snap(
            None,
            &format!("http://127.0.0.1:{port}/x"),
            &dir,
            &want(&sha(b"x"), 1),
            false,
        )
        .unwrap();
        h.join().unwrap();
        assert_eq!(got, Err(403));
        fs::remove_dir_all(&dir).unwrap();
    }

    fn fixture(name: &str) -> String {
        let p = format!(
            "{}/src/testdata/assertions/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        fs::read_to_string(p).unwrap()
    }

    /// The recorded hello-world 29 assertions, served by path.
    struct Recorded(HashMap<String, String>);

    impl Source for Recorded {
        fn fetch(&mut self, path: &str) -> Result<String, StoreError> {
            let key = path.split('?').next().unwrap_or(path);
            self.0
                .get(key)
                .cloned()
                .ok_or_else(|| StoreError::NotFound(format!("no {path}")))
        }
    }

    fn recorded() -> (Recorded, String) {
        let hex = fixture("hello-world-29.sha3-384").trim().to_string();
        let mut m = HashMap::new();
        m.insert(
            format!("snap-revision/{}", assertion::sha3_key(&hex)),
            fixture("snap-revision-hello-world-29"),
        );
        m.insert(
            "snap-declaration/16/buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ".into(),
            fixture("snap-declaration-hello-world"),
        );
        m.insert(
            "account-key/BWDEoaqyr25nF5SNCvEv2v7QnM9QsfCc0PBMYD_i2NGSQ32EF2d4D0hqUel3m8ul".into(),
            fixture("account-key-store"),
        );
        m.insert("account/canonical".into(), fixture("account-canonical"));
        (Recorded(m), hex)
    }

    #[test]
    fn assertions_for_a_revision() {
        let (mut src, hex) = recorded();
        let path = format!("snap-revision/{}", assertion::sha3_key(&hex));
        let got = assertion::fetch(&mut src, &path, true).unwrap();
        let keys: Vec<String> = got
            .iter()
            .map(|a| {
                let h = assertion::headers(a).0;
                h.iter().find(|(k, _)| k == "type").unwrap().1.clone()
            })
            .collect();
        // each after what it needs, once
        assert_eq!(keys, ["account-key", "snap-declaration", "snap-revision"]);
        let rev = assertion::headers(&got[2]).0;
        let size: u64 = rev
            .iter()
            .find(|(k, _)| k == "snap-size")
            .unwrap()
            .1
            .parse()
            .unwrap();
        let w = Expected {
            name: "hello-world",
            revision: 29,
            size,
            sha3_384: &hex,
        };
        check_revision(&got, "buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ", &w).unwrap();
        let wrong = Expected {
            size: size + 1,
            ..w
        };
        let e = check_revision(&got, "buPKUD3TKqCOgLEjjHx5kSiCpIs5cMuQ", &wrong).unwrap_err();
        assert!(e.to_string().contains("snap-size"), "{e}");
    }

    #[test]
    fn stale_partials_removed() {
        let dir = scratch("stale");
        private_dir(&dir).unwrap();
        // pid 0 is never a process in /proc; ours is
        fs::write(dir.join(".x_1.snap.0.partial"), b"").unwrap();
        let mine = partial_path(&dir, "y_1.snap");
        fs::write(&mine, b"").unwrap();
        clean_stale_partials(&dir);
        assert!(!dir.join(".x_1.snap.0.partial").exists());
        assert!(mine.exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
