//! idiot-store - the Snap Store calls idiot can't make from sh.
//!
//! A leaf tool for idiot's `auth` commands: it only talks to the store.
//! Credentials never go through argv (visible in ps) or stdout: `login`
//! writes a private file, `whoami` reads the environment or a file.
//! `expiry` reads a credential's expiry caveats offline and prints only
//! those timestamps; `repair` strips base64 padding snapd can't read from a
//! credential file, in place, and prints only `ok`, `repaired` or `padded`.
//! `permissions`, `account`, `store-snaps`, `store-members`, `info` and
//! `probe` are the
//! read-only queries behind `idiot store`, printing tab-separated lines.
//! `assertion` fetches an assertion (and with `--chain` its prerequisites)
//! and prints the stream, only once every assertion has been fetched.
//!
//! Exit status: 0 ok, 1 usage or local error, 2 authentication failed or
//! credential rejected, 3 network or store error, 4 not found (or hidden
//! from this credential; `assertion` only).

use clap::{Args, Parser, Subcommand};
use std::process::ExitCode;

mod cmd;

/// Permissions `snapcraft export-login` asks for by default.
const DEFAULT_PERMISSIONS: &str = "package_access,package_manage,package_metrics,package_push,package_register,package_release,package_update";

#[derive(Parser)]
#[command(
    name = "idiot-store",
    version,
    about = "Snap Store login and whoami for idiot"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Log in on the terminal and write a new credential to a private file
    Login(LoginArgs),
    /// Show the account behind a credential, as `snapcraft whoami` does
    Whoami(WhoamiArgs),
    /// Print a credential's expiry caveats (offline): `root|discharge <time>`
    Expiry(ExpiryArgs),
    /// Strip base64 padding (which snapd can't read) from a credential file, in
    /// place (offline): prints ok, repaired, or with --check padded
    Repair(RepairArgs),
    /// A credential's permissions (offline), one per line
    Permissions(CredArgs),
    /// The account's stores (`store <id> <name> <roles>`) and snaps
    /// (`snap <name> <store name> <private|public> <publisher>`)
    Account(CredArgs),
    /// A brand store's snaps (store admins with store_admin):
    /// `snap <name> <owner> <private|public>`, or `refused ...`
    StoreSnaps(StoreSnapsArgs),
    /// A brand store's members (store admins with store_admin):
    /// `member <username> <roles> <display name> <email>`, or `refused ...`
    StoreMembers(StoreSnapsArgs),
    /// A snap's channel map: `channel <track/risk> <arch> <revision> <version> <released-at>`
    Info(InfoArgs),
    /// Whether a snap revision can be downloaded, anonymously and with the
    /// credential (statuses only; nothing is downloaded)
    Probe(ProbeArgs),
    /// Fetch an assertion by type and primary-key headers (header=value;
    /// snap-name= resolves snap-id, with snap-revision= a revision's
    /// sha3-384) and print the stream; --chain adds its prerequisites
    Assertion(AssertionArgs),
}

#[derive(Args)]
pub struct AssertionArgs {
    #[command(flatten)]
    cred: CredArgs,
    /// The store's ID, sent as Snap-Device-Store (default: the global store)
    #[arg(long)]
    store: Option<String>,
    /// Architecture for revision lookups outside the channel map
    #[arg(long)]
    arch: Option<String>,
    /// Also fetch what snapd needs before it accepts the assertion
    #[arg(long)]
    chain: bool,
    /// The assertion type, e.g. account, snap-revision
    kind: String,
    /// Primary-key headers: header=value
    headers: Vec<String>,
}

#[derive(Args)]
pub struct CredArgs {
    /// Read the credential from FILE instead of UBUNTU_STORE_AUTH
    #[arg(long, value_name = "FILE")]
    file: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct StoreSnapsArgs {
    #[command(flatten)]
    cred: CredArgs,
    /// The store's ID
    #[arg(long)]
    store: String,
}

#[derive(Args)]
pub struct InfoArgs {
    #[command(flatten)]
    cred: CredArgs,
    /// The store's ID (default: the global store)
    #[arg(long)]
    store: Option<String>,
    snap: String,
}

#[derive(Args)]
pub struct ProbeArgs {
    #[command(flatten)]
    cred: CredArgs,
    /// The store's ID (default: the global store)
    #[arg(long)]
    store: Option<String>,
    /// Architecture, e.g. arm64
    #[arg(long)]
    arch: String,
    /// Channel, e.g. stable or 2.0/edge
    #[arg(long, default_value = "stable")]
    channel: String,
    snap: String,
}

#[derive(Args)]
pub struct LoginArgs {
    /// File to create (mode 0600; must not exist)
    #[arg(long, value_name = "FILE")]
    output: std::path::PathBuf,
    /// Comma-separated store permissions
    #[arg(long, value_delimiter = ',', default_value = DEFAULT_PERMISSIONS)]
    permissions: Vec<String>,
    /// Comma-separated snap names to restrict the credential to
    #[arg(long, value_delimiter = ',')]
    snaps: Vec<String>,
    /// Comma-separated channels to restrict the credential to
    #[arg(long, value_delimiter = ',')]
    channels: Vec<String>,
    /// Expiry, RFC 3339 (e.g. 2027-11-09T00:00:00+00:00)
    #[arg(long, value_name = "WHEN")]
    expires: String,
    /// Shown on the store dashboard next to the credential
    #[arg(long, default_value = "idiot-store")]
    description: String,
}

#[derive(Args)]
pub struct WhoamiArgs {
    /// Read the credential from FILE instead of UBUNTU_STORE_AUTH
    #[arg(long, value_name = "FILE")]
    file: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct ExpiryArgs {
    /// Read the credential from FILE instead of UBUNTU_STORE_AUTH
    #[arg(long, value_name = "FILE")]
    file: Option<std::path::PathBuf>,
}

#[derive(Args)]
pub struct RepairArgs {
    /// Only report: print `padded` instead of repairing
    #[arg(long)]
    check: bool,
    /// The credential file
    file: std::path::PathBuf,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            let _ = e.print();
            return ExitCode::from(if e.use_stderr() { 1 } else { 0 });
        }
    };
    if macaroon::initialize().is_err() {
        eprintln!("idiot-store: cannot initialise crypto");
        return ExitCode::from(1);
    }
    let result = match cli.command {
        Commands::Login(args) => cmd::login::run(args),
        Commands::Whoami(args) => cmd::whoami::run(args),
        Commands::Expiry(args) => cmd::expiry::run(args),
        Commands::Repair(args) => cmd::repair::run(args),
        Commands::Permissions(args) => cmd::query::run_permissions(args),
        Commands::Account(args) => cmd::query::run_account(args),
        Commands::StoreSnaps(args) => cmd::query::run_store_snaps(args),
        Commands::StoreMembers(args) => cmd::query::run_store_members(args),
        Commands::Info(args) => cmd::query::run_info(args),
        Commands::Probe(args) => cmd::query::run_probe(args),
        Commands::Assertion(args) => cmd::query::run_assertion(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("idiot-store: {e}");
            ExitCode::from(e.code)
        }
    }
}
