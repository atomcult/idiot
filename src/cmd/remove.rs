use anyhow::Result;
use async_trait::async_trait;
use clap::{ArgMatches, Command};

use super::Cmd;

const NAME: &'static str = "rm";

#[derive(Default)]
pub struct CmdRemove;

impl CmdRemove {
    pub fn init() -> (String, Command, Box<dyn Cmd>) {
        (NAME.into(), Self::cmd(), Box::new(CmdRemove {}))
    }

    fn cmd() -> Command {
        Command::new(NAME).about("Remove saved credentials")
    }
}

#[async_trait]
impl Cmd for CmdRemove {
    async fn run<'a, 'b>(&'a self, _args: &'b ArgMatches) -> Result<()> {
        println!("'{}' command under construction!", NAME);
        Ok(())
    }
}
