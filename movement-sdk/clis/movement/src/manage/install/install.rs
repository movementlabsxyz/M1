use async_trait::async_trait;
use clap::{Subcommand, Parser};

#[derive(Debug, Parser)]
pub struct All;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Install Movement artifacts"
)]
pub enum Install {
    All(All)
}