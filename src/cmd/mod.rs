use anyhow::Result;
use clap::{ArgMatches, Command};

pub mod login;

pub struct SubCmd {
    pub name: String,
    pub cmd: fn() -> Command,
    pub run: fn(&ArgMatches) -> Result<()>,
}
