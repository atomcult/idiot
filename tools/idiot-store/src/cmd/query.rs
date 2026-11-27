//! The read-only store queries, printed as tab-separated lines on stdout.
//! Credentials come from `--file` or `UBUNTU_STORE_AUTH`, never argv, and
//! are never printed.

use super::Failure;
use crate::{
    AssertionArgs, ChannelMapsArgs, CredArgs, InfoArgs, ProbeArgs, ReleasesArgs, StoreSnapsArgs,
};
use idiot_store::store::assertion::{self, Fetcher};
use idiot_store::store::credential::permissions;
use idiot_store::store::explore::{
    self, Endpoints, HistoryRefusal, Release, Revision, SharedAuth, SnapMeta,
};
use idiot_store::store::query::{self, Auth, Channel, Refusal, StoreMember, StoreSnap};

/// The credential from --file, else UBUNTU_STORE_AUTH; None when neither.
pub(super) fn credential(args: &CredArgs) -> Result<Option<String>, Failure> {
    match &args.file {
        Some(path) => std::fs::read_to_string(path)
            .map(Some)
            .map_err(|e| Failure::local(format!("{}: {e}", path.display()))),
        None => Ok(std::env::var("UBUNTU_STORE_AUTH")
            .ok()
            .filter(|v| !v.is_empty())),
    }
}

fn required(args: &CredArgs) -> Result<String, Failure> {
    credential(args)?.ok_or_else(|| Failure::local("UBUNTU_STORE_AUTH is not set (or use --file)"))
}

fn flag(private: bool) -> &'static str {
    if private {
        "private"
    } else {
        "public"
    }
}

pub fn run_permissions(args: CredArgs) -> Result<(), Failure> {
    for p in permissions(&required(&args)?)? {
        println!("{p}");
    }
    Ok(())
}

pub fn run_account(args: CredArgs) -> Result<(), Failure> {
    let mut auth = Auth::new(&required(&args)?);
    let (stores, snaps) = query::account(&mut auth)?;
    for s in stores {
        println!("store\t{}\t{}\t{}", s.id, s.name, s.roles.join(","));
    }
    for s in snaps {
        println!(
            "snap\t{}\t{}\t{}\t{}",
            s.name,
            s.store,
            flag(s.private),
            s.publisher
        );
    }
    Ok(())
}

pub fn run_store_snaps(args: StoreSnapsArgs) -> Result<(), Failure> {
    let mut auth = Auth::new(&required(&args.cred)?);
    match query::store_snaps(&mut auth, &args.store)? {
        Ok(snaps) => {
            for s in &snaps {
                println!("{}", store_snap_line(s));
            }
        }
        Err(r) => println!("{}", refusal_line(r)),
    }
    Ok(())
}

/// A `store-snaps` line: the first three fields as they always were, then
/// the snap ID and the latest release (`-` without one).
fn store_snap_line(s: &StoreSnap) -> String {
    let (ch, rev, ver) = match &s.latest {
        Some((c, r, v)) => (dash(c), r.to_string(), dash(v)),
        None => ("-".into(), "-".into(), "-".into()),
    };
    format!(
        "snap\t{}\t{}\t{}\t{}\t{ch}\t{rev}\t{ver}",
        s.name,
        s.owner,
        flag(s.private),
        dash(&s.snap_id)
    )
}

/// `s` as one field, `-` when empty.
fn dash(s: &str) -> String {
    if s.is_empty() {
        "-".into()
    } else {
        field(s)
    }
}

/// `s` with tabs and newlines as spaces, so it stays one field.
fn field(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

/// A `member` line: username, roles, display name, email (no account ID).
fn member_line(m: &StoreMember) -> String {
    format!(
        "member\t{}\t{}\t{}\t{}",
        field(&m.username),
        m.roles.join(","),
        field(&m.displayname),
        field(&m.email)
    )
}

fn refusal_line(r: Refusal) -> String {
    match r {
        Refusal::Permission(p) => format!("refused\tpermission\t{p}"),
        Refusal::NotAdmin => "refused\tnot-admin".to_string(),
    }
}

pub fn run_store_members(args: StoreSnapsArgs) -> Result<(), Failure> {
    let mut auth = Auth::new(&required(&args.cred)?);
    match query::store_members(&mut auth, &args.store)? {
        Ok(members) => {
            for m in &members {
                println!("{}", member_line(m));
            }
        }
        Err(r) => println!("{}", refusal_line(r)),
    }
    Ok(())
}

pub fn run_info(args: InfoArgs) -> Result<(), Failure> {
    let mut auth = credential(&args.cred)?.map(|c| Auth::new(&c));
    match query::info(auth.as_mut(), &args.snap, args.store.as_deref())? {
        Some(map) => {
            for c in &map {
                println!("{}", info_line(c));
            }
            Ok(())
        }
        None => Err(Failure {
            code: 3,
            message: format!("{}: not found in this store", args.snap),
        }),
    }
}

/// An `info` line, as `store channels` and `store check` read it.
fn info_line(c: &Channel) -> String {
    format!(
        "channel\t{}\t{}\t{}\t{}\t{}",
        c.name, c.arch, c.revision, c.version, c.released_at
    )
}

fn snap_line(name: &str, m: &SnapMeta) -> String {
    format!(
        "snap\t{name}\t{}\t{}\t{}\t{}",
        dash(&m.snap_id),
        dash(&m.publisher),
        flag(m.private),
        field(&m.title)
    )
}

fn map_line(name: &str, c: &Channel) -> String {
    format!(
        "channel\t{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        c.name,
        c.arch,
        c.revision,
        dash(&c.version),
        dash(&c.released_at),
        dash(&c.created_at),
        dash(&c.kind),
        c.base.as_deref().map_or("-".into(), dash),
        dash(&c.confinement),
        dash(&c.epoch),
        c.size,
        dash(&c.sha3_384)
    )
}

fn revision_line(r: &Revision) -> String {
    format!(
        "revision\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        r.revision,
        dash(&r.version),
        dash(&r.architectures.join(",")),
        dash(&r.status),
        dash(&r.created_at),
        r.base.as_deref().map_or("-".into(), dash),
        dash(&r.confinement),
        dash(&r.grade),
        dash(&r.epoch),
        r.size,
        dash(&r.sha3_384)
    )
}

fn release_line(r: &Release) -> String {
    let pct = r.progressive.map_or("-".into(), |p| {
        if p.fract() == 0.0 {
            format!("{p:.0}")
        } else {
            format!("{p}")
        }
    });
    format!(
        "release\t{}\t{}\t{}\t{}\t{}\t{pct}",
        r.channel,
        dash(&r.arch),
        r.revision.map_or("-".into(), |n| n.to_string()),
        dash(&r.when),
        r.expires.as_deref().map_or("-".into(), dash)
    )
}

pub fn run_channel_maps(args: ChannelMapsArgs) -> Result<(), Failure> {
    let auth = credential(&args.cred)?.map(|c| SharedAuth::new(&c));
    let maps = explore::channel_maps(
        &Endpoints::default(),
        auth.as_ref(),
        &args.snaps,
        args.store.as_deref(),
    )?;
    for (name, got) in args.snaps.iter().zip(maps) {
        match got {
            Some((meta, map)) => {
                println!("{}", snap_line(name, &meta));
                for c in &map {
                    println!("{}", map_line(name, c));
                }
            }
            None => println!("hidden\t{name}"),
        }
    }
    Ok(())
}

pub fn run_releases(args: ReleasesArgs) -> Result<(), Failure> {
    let auth = credential(&args.cred)?.map(|c| SharedAuth::new(&c));
    let answer = explore::releases(&Endpoints::default(), auth.as_ref(), &args.snap)?;
    if let Some(note) = answer.stopped {
        eprintln!("idiot-store: {note}");
    }
    match answer.history {
        Ok(h) => {
            for r in &h.revisions {
                println!("{}", revision_line(r));
            }
            for r in &h.releases {
                println!("{}", release_line(r));
            }
        }
        Err(HistoryRefusal::Permission(p)) => println!("refused\tpermission\t{p}"),
        Err(HistoryRefusal::NotCollaborator) => println!("refused\tnot-collaborator"),
        Err(HistoryRefusal::NoCredential) => println!("refused\tno-credential"),
    }
    Ok(())
}

pub fn run_probe(args: ProbeArgs) -> Result<(), Failure> {
    let mut auth = credential(&args.cred)?.map(|c| Auth::new(&c));
    for line in query::probe(
        auth.as_mut(),
        &args.snap,
        args.store.as_deref(),
        &args.channel,
        &args.arch,
    )? {
        println!("{line}");
    }
    Ok(())
}

pub fn run_assertion(args: AssertionArgs) -> Result<(), Failure> {
    let req = assertion::parse(&args.kind, &args.headers).map_err(Failure::local)?;
    let mut auth = credential(&args.cred)?.map(|c| Auth::new(&c));
    let mut fetcher = Fetcher::new(auth.as_mut(), args.store, args.arch)?;
    let req = fetcher.resolve(req)?;
    let got = assertion::fetch(&mut fetcher, &assertion::path(&req), args.chain)?;
    print!("{}", assertion::stream(&got));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use idiot_store::store::query::{parse_store_members, Reply};
    use reqwest::StatusCode;

    fn fixture(name: &str) -> String {
        let path = format!("{}/src/testdata/store/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn member_lines_without_account_ids() {
        let reply = Reply {
            status: StatusCode::OK,
            body: fixture("store-members.json"),
            wants_refresh: false,
        };
        let lines: Vec<String> = parse_store_members(&reply)
            .unwrap()
            .unwrap()
            .iter()
            .map(member_line)
            .collect();
        assert_eq!(
            lines[0],
            "member\talice-example\tadmin,review,view\tAlice Example\talice@example.com"
        );
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|l| !l.contains("AccountId")));
    }

    #[test]
    fn member_fields_stay_on_one_line() {
        let m = StoreMember {
            username: "x".into(),
            roles: vec!["view".into()],
            displayname: "A\tB\nC".into(),
            email: String::new(),
        };
        assert_eq!(member_line(&m), "member\tx\tview\tA B C\t");
    }

    fn json(name: &str) -> serde_json::Value {
        serde_json::from_str(&fixture(name)).unwrap()
    }

    #[test]
    fn info_lines_unchanged() {
        let lines: Vec<String> = query::parse_info(&json("info.json"))
            .iter()
            .map(info_line)
            .collect();
        assert_eq!(
            lines,
            [
                "channel\tlatest/edge\tarm64\t4\t0.4\t2026-10-03T10:00:00.000000+00:00",
                "channel\tlatest/stable\tamd64\t3\t0.3\t2026-10-02T10:00:00.000000+00:00",
            ]
        );
    }

    #[test]
    fn channel_map_lines() {
        let j = json("channel-map.json");
        let meta = explore::parse_meta(&j);
        assert_eq!(
            snap_line("ce-dummy", &meta),
            "snap\tce-dummy\tSnapIdCeDummyXXXXXXXXXXXXXXXXXXX\ttest-user\tprivate\tCE dummy"
        );
        let map = query::parse_info(&j);
        let edge = map.iter().find(|c| c.name == "latest/edge").unwrap();
        assert_eq!(
            map_line("ce-dummy", edge),
            format!("channel\tce-dummy\tlatest/edge\tarm64\t4\t0.4\t2026-10-03T10:00:00.000000+00:00\t2026-10-03T09:00:00.000000+00:00\tapp\tcore24\tstrict\t1*\t8192\t{}", "ab".repeat(48))
        );
        let base = query::parse_info(&json("channel-map-base.json"));
        assert_eq!(
            map_line("example-base", &base[0]).split('\t').nth(9),
            Some("-")
        );
    }

    #[test]
    fn release_lines() {
        let mut h = explore::History::default();
        explore::parse_releases_page(&json("releases.json"), &mut h);
        let unreleased = h.revisions.iter().find(|r| r.revision == 5).unwrap();
        assert_eq!(
            revision_line(unreleased),
            format!("revision\t5\t0.5\tamd64,arm64\tPublished\t2026-10-05T09:00:00Z\tcore24\tstrict\tdevel\t1*\t8192\t{}", "cd".repeat(48))
        );
        let lines: Vec<String> = h.releases.iter().map(release_line).collect();
        assert!(lines
            .contains(&"release\tlatest/beta\tamd64\t-\t2026-10-02T11:00:00Z\t-\t-".to_string()));
        assert!(lines
            .contains(&"release\tlatest/edge\tarm64\t4\t2026-10-03T10:00:00Z\t-\t30".to_string()));
        assert!(lines.contains(
            &"release\tlatest/edge/fix-1\tarm64\t4\t2026-10-04T10:00:00Z\t2026-11-03T10:00:00Z\t-"
                .to_string()
        ));
    }

    #[test]
    fn store_snap_lines_keep_their_first_fields() {
        let reply = Reply {
            status: StatusCode::OK,
            body: fixture("store-snaps.json"),
            wants_refresh: false,
        };
        let lines: Vec<String> = query::parse_store_snaps(&reply)
            .unwrap()
            .unwrap()
            .iter()
            .map(store_snap_line)
            .collect();
        assert_eq!(
            lines,
            [
                "snap\tcore\tpublisher\tpublic\tSnapIdCoreXXXXXXXXXXXXXXXXXXXXXX\tstable\t1\t1",
                "snap\thello-team\ttest-user\tprivate\tSnapIdHelloTeamXXXXXXXXXXXXXXXXX\tstable\t3\t0.3",
            ]
        );
        let none = StoreSnap {
            name: "x".into(),
            owner: "o".into(),
            private: false,
            snap_id: String::new(),
            latest: None,
        };
        assert_eq!(store_snap_line(&none), "snap\tx\to\tpublic\t-\t-\t-\t-");
    }

    #[test]
    fn refusals_printed() {
        assert_eq!(
            refusal_line(Refusal::Permission("store_admin".into())),
            "refused\tpermission\tstore_admin"
        );
        assert_eq!(refusal_line(Refusal::NotAdmin), "refused\tnot-admin");
    }
}
