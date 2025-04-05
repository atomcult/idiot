use anyhow::Result;
use async_trait::async_trait;
use clap::{ArgMatches, Command};

use super::Cmd;

const NAME: &'static str = "login";

#[derive(Default)]
pub struct CmdLogin;

impl CmdLogin {
    pub fn init() -> (String, Command, Box<dyn Cmd>) {
        (NAME.into(), Self::cmd(), Box::new(CmdLogin {}))
    }

    fn cmd() -> Command {
        Command::new(NAME).about("Log into a SaaS Store account (default)")
    }
}

#[async_trait]
impl Cmd for CmdLogin {
    async fn run<'a, 'b>(&'a self, _args: &'b ArgMatches) -> Result<()> {
        println!("'{}' command under construction!", NAME);
        Ok(())
    }
}
