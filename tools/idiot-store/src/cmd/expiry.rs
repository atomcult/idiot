use super::Failure;
use crate::ExpiryArgs;
use idiot_store::store::credential::expiries;

/// Prints `root <time>` / `discharge <time>` for each expiry caveat, one
/// per line, and nothing else. Never contacts the network.
pub fn run(args: ExpiryArgs) -> Result<(), Failure> {
    let credential = match args.file {
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|e| Failure::local(format!("{}: {e}", path.display())))?,
        None => std::env::var("UBUNTU_STORE_AUTH")
            .map_err(|_| Failure::local("UBUNTU_STORE_AUTH is not set (or use --file)"))?,
    };
    for (label, time) in expiries(&credential)? {
        println!("{label} {time}");
    }
    Ok(())
}
