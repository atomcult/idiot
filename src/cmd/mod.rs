use anyhow::Result;
use async_trait::async_trait;
use clap::ArgMatches;

mod add;
mod list;
mod login;
mod remove;

pub type CmdAdd = add::CmdAdd;
pub type CmdLogin = login::CmdLogin;
pub type CmdList = list::CmdList;
pub type CmdRemove = remove::CmdRemove;

#[async_trait]
pub trait Cmd {
    async fn run<'a, 'b>(&'a self, args: &'b ArgMatches) -> Result<()>;
}
