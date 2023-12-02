use async_trait::async_trait;
use clap::Subcommand;
use super::{
    start::Start,
    status::Status,
    stop::Stop,
};

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Control Movement services"
)]
pub enum Ctl {
    #[clap(subcommand)]
    Start(Start),
    #[clap(subcommand)]
    Status(Status),
    #[clap(subcommand)]
    Stop(Stop),
}