use anyhow::Result;
use async_trait::async_trait;
use clap::ArgMatches;

pub mod login;

#[async_trait]
pub trait Cmd {
    async fn run<'a, 'b>(&'a self, args: &'b ArgMatches) -> Result<()>;
}
