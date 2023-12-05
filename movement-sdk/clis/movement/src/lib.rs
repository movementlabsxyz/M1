pub mod foo;
pub mod common;
pub mod manage;
pub mod ctl;
// pub mod aptos;
// pub mod sui;
pub mod canonical;

use clap::*;

use async_trait::async_trait;
use foo::Foo;
use manage::Manage;
use ctl::Ctl;
use sui::sui_commands::SuiCommand;
use aptos::Tool;
use crate::common::cli::Command;
// use crate::aptos::Aptos;
// use crate::sui::Sui;

const VERSION: &str = const_str::concat!(env!("CARGO_PKG_VERSION"));


#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum MovementCommand {
    #[clap(subcommand)]
    Foo(Foo),
    #[clap(subcommand)]
    Manage(Manage),
    #[clap(subcommand)]
    Ctl(Ctl),
    #[clap(subcommand, about = "Run Aptos commands")]
    Aptos(Tool),
    #[clap(subcommand, about = "Run Sui commands")]
    Sui(SuiCommand)
}

#[async_trait::async_trait]
impl Command<String> for MovementCommand {

    async fn get_name(&self) -> String {
        "movement".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

      match self {
        MovementCommand::Manage(manage) => {
            Ok("SUCCESS".to_string())
        },
        MovementCommand::Ctl(ctl) => {
            // ctl.execute().await?;
            Ok("SUCCESS".to_string())
        },
        MovementCommand::Aptos(aptos) => {
            aptos.execute().await.map_err(
               |e| anyhow::anyhow!("aptos error: {:?}", e)
            )?;
            Ok("SUCCESS".to_string())
        },
        MovementCommand::Sui(sui) => {
            sui.execute().await?;
            Ok("SUCCESS".to_string())
        },
        _ => {
            Ok("NOT FOUND".to_string())
        }
      }

    }

}

#[derive(Parser)]
#[clap(
    // name = env!("CARGO_BIN_NAME"),
    name = "movement",
    about = "A network of Move-based blockchains",
    rename_all = "kebab-case",
    author,
    version = VERSION,
    propagate_version = true,
)]
pub struct Movement {
    #[clap(subcommand)]
    pub command: MovementCommand
}