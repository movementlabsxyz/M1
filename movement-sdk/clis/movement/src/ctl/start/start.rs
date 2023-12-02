use async_trait::async_trait;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Start a Movement service"
)]
pub enum Start {
    
}