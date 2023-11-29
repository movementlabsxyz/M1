pub mod common;

pub mod manage;
pub mod ctl;
pub mod aptos;
pub mod sui;
pub mod canonical;

use async_trait::async_trait;
use clap::Parser;
use ctl::ctl::Ctl;

#[derive(Parser)]
#[clap(name = "movement", author, version, propagate_version = true)]
pub enum Movement {

}