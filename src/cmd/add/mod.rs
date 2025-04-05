use anyhow::Result;
use async_trait::async_trait;
use clap::{ArgMatches, Command};

mod ui;

use super::Cmd;

const NAME: &'static str = "add";

#[derive(Default)]
pub struct CmdAdd;

impl CmdAdd {
    pub fn init() -> (String, Command, Box<dyn Cmd>) {
        (NAME.into(), Self::cmd(), Box::new(CmdAdd {}))
    }

    pub fn cmd() -> Command {
        Command::new(NAME).about("Log into the SaaS snap store (default)")
    }
}

#[async_trait]
impl Cmd for CmdAdd {
    async fn run<'a, 'b>(&'a self, _args: &'b ArgMatches) -> Result<()> {
        ui::run().await?;
        Ok(())
    }
}
