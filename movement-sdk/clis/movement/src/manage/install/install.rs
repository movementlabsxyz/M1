use async_trait::async_trait;
use clap::{Subcommand, Parser};

#[derive(Debug, Parser)]
pub struct All;

#[derive(Subcommand, Debug)]
pub enum Install {
    All(All)
}