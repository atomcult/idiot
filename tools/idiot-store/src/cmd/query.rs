//! The read-only store queries, printed as tab-separated lines on stdout.
//! Credentials come from `--file` or `UBUNTU_STORE_AUTH`, never argv, and
//! are never printed.

use super::Failure;
use crate::{AssertionArgs, CredArgs, InfoArgs, ProbeArgs, StoreSnapsArgs};
use idiot_store::store::assertion::{self, Fetcher};
use idiot_store::store::credential::permissions;
use idiot_store::store::query::{self, Auth, Refusal, StoreMember};

/// The credential from --file, else UBUNTU_STORE_AUTH; None when neither.
fn credential(args: &CredArgs) -> Result<Option<String>, Failure> {
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
            for s in snaps {
                println!("snap\t{}\t{}\t{}", s.name, s.owner, flag(s.private));
            }
        }
        Err(r) => println!("{}", refusal_line(r)),
    }
    Ok(())
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
            for c in map {
                println!(
                    "channel\t{}\t{}\t{}\t{}\t{}",
                    c.name, c.arch, c.revision, c.version, c.released_at
                );
            }
            Ok(())
        }
        None => Err(Failure {
            code: 3,
            message: format!("{}: not found in this store", args.snap),
        }),
    }
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

    #[test]
    fn refusals_printed() {
        assert_eq!(
            refusal_line(Refusal::Permission("store_admin".into())),
            "refused\tpermission\tstore_admin"
        );
        assert_eq!(refusal_line(Refusal::NotAdmin), "refused\tnot-admin");
    }
}
