//! `download`: a released snap revision for an architecture, verified, into
//! a directory, with its assertions; one result line for sh on stdout.

use super::query::credential;
use super::Failure;
use crate::DownloadArgs;
use idiot_store::store::download::{self, How, Outcome, Request};
use idiot_store::store::query::Auth;

pub fn run(args: DownloadArgs) -> Result<(), Failure> {
    let mut auth = credential(&args.cred)?.map(|c| Auth::new(&c));
    let rq = Request {
        snap: &args.snap,
        store: args.store.as_deref(),
        arch: &args.arch,
        channel: &args.channel,
        dir: &args.dir,
        assertions: !args.no_assertions,
        progress: args.progress,
    };
    match download::download(auth.as_mut(), &rq)? {
        Outcome::Snap(f) => println!(
            "snap\t{}\t{}\t{}\t{}\t{}",
            f.path.display(),
            f.name,
            f.revision,
            f.version,
            match f.how {
                How::Downloaded => "downloaded",
                How::Cached => "cached",
            }
        ),
        Outcome::Unknown => println!("unknown"),
        Outcome::Unreleased {
            channel,
            arch,
            available,
        } => {
            println!("unreleased\t{channel}\t{arch}");
            for (c, r) in available {
                println!("available\t{c}\t{r}");
            }
        }
        Outcome::Refused(status) => println!("refused\t{status}"),
    }
    Ok(())
}
