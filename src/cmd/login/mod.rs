use anyhow::Result;
use clap::{ArgMatches, Command};

mod ui;

const NAME: &'static str = "login";

pub fn init() -> super::SubCmd {
    super::SubCmd {
        name: NAME.into(),
        cmd,
        run,
    }
}

pub fn cmd() -> Command {
    Command::new(NAME).about("Log into the SaaS snap store")
}

pub fn run(_args: &ArgMatches) -> Result<()> {
    ui::spawn();
    Ok(())
}
