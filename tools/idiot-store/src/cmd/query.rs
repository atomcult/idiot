//! The read-only store queries, printed as tab-separated lines on stdout.
//! Credentials come from `--file` or `UBUNTU_STORE_AUTH`, never argv, and
//! are never printed.

use super::Failure;
use crate::{AssertionArgs, CredArgs, InfoArgs, ProbeArgs, StoreSnapsArgs};
use idiot_store::store::assertion::{self, Fetcher};
use idiot_store::store::credential::permissions;
use idiot_store::store::query::{self, Auth, Refusal};

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
        Err(Refusal::Permission(p)) => println!("refused\tpermission\t{p}"),
        Err(Refusal::NotAdmin) => println!("refused\tnot-admin"),
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
