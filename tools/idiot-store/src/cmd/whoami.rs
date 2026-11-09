use super::Failure;
use crate::WhoamiArgs;
use idiot_store::store::whoami::whoami;

pub fn run(args: WhoamiArgs) -> Result<(), Failure> {
    let credential = match args.file {
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|e| Failure::local(format!("{}: {e}", path.display())))?,
        None => std::env::var("UBUNTU_STORE_AUTH")
            .map_err(|_| Failure::local("UBUNTU_STORE_AUTH is not set (or use --file)"))?,
    };
    print!("{}", whoami(&credential)?.report());
    Ok(())
}
