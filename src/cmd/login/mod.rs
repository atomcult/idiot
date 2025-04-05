use anyhow::Result;
use async_trait::async_trait;
use clap::{ArgMatches, Command};

mod add;

use super::Cmd;

const NAME: &'static str = "login";

pub fn init() -> (String, Command, Box<impl Cmd>) {
    (NAME.into(), cmd(), Box::new(Login {}))
}

pub fn cmd() -> Command {
    Command::new(NAME).about("Log into the SaaS snap store")
}

#[derive(Default)]
pub struct Login;

#[async_trait]
impl Cmd for Login {
    async fn run<'a, 'b>(&'a self, _args: &'b ArgMatches) -> Result<()> {
        add::run().await?;
        Ok(())
    }
}
