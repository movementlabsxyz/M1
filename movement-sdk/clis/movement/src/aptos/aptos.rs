use clap::Parser;
use aptos::Tool;
use clap::Subcommand;
use std::fmt::{self, Debug, Formatter};
use crate::common::cli::Command;

#[derive(Subcommand)]
pub enum Aptos {
    #[clap(subcommand)]
    Tool(Tool)
}

impl Debug for Aptos {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Aptos")
    }
}