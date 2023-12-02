use async_trait::async_trait;
use clap::Subcommand;
use super::install::Install;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Manage the Movement CLI and artifacts"
)]
pub enum Manage {
    #[clap(subcommand)]
    Install(Install)
}