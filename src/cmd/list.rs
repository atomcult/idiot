use anyhow::Result;
use async_trait::async_trait;
use clap::{ArgMatches, Command};

use super::Cmd;

const NAME: &'static str = "list";

#[derive(Default)]
pub struct CmdList;

impl CmdList {
    pub fn init() -> (String, Command, Box<dyn Cmd>) {
        (NAME.into(), Self::cmd(), Box::new(CmdList {}))
    }

    fn cmd() -> Command {
        Command::new(NAME).about("List saved credentials")
    }
}

#[async_trait]
impl Cmd for CmdList {
    async fn run<'a, 'b>(&'a self, _args: &'b ArgMatches) -> Result<()> {
        println!("'{}' command under construction!", NAME);
        Ok(())
    }
}
