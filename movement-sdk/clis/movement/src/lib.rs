pub mod common;

pub mod manage;
pub mod ctl;
pub mod aptos;
pub mod sui;
pub mod canonical;

use async_trait::async_trait;
use clap::Parser;
use ctl::Ctl;

#[derive(Parser, Debug)]
#[clap(name = "movement", author, version, propagate_version = true)]
pub enum Movement {
    #[clap(subcommand)]
    Ctl(Ctl),
}