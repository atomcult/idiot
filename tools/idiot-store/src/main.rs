use clap::{Parser, Subcommand};
use std::error::Error;

mod cmd;

#[derive(Parser)]
#[command(name = "idiot-store", about = "Interactive Devtools for IoT")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate interactively and write a store credential to stdout
    Login,
    /// Show details of the account behind UBUNTU_STORE_AUTH
    Whoami,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Login => cmd::login::run().await,
        Commands::Whoami => cmd::whoami::run().await,
    }
}
