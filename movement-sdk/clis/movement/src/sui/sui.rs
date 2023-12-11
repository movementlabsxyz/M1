pub use clap::Parser;
use sui::sui_commands::SuiCommand;
use clap::{FromArgMatches, ArgMatches, Subcommand};
use std::fmt::{self, Debug, Formatter};
use crate::common::cli::Command;



#[derive(Subcommand)]
pub struct Sui {
    #[clap(subcommand)]
    command: SuiCommand
}

impl Debug for Sui {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Sui")
    }
}

#[async_trait::async_trait]
impl Command<String> for Sui {

    async fn get_name(&self) -> String {
        "sui".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {
        #[cfg(windows)]
        colored::control::set_virtual_terminal(true).unwrap();

        let args = Args::parse();
        let _guard = match args.command {
            SuiCommand::Console { .. }
            | SuiCommand::Client { .. }
            | SuiCommand::KeyTool { .. }
            | SuiCommand::Move { .. } => telemetry_subscribers::TelemetryConfig::new()
                .with_log_level("error")
                .with_env()
                .init(),
            _ => telemetry_subscribers::TelemetryConfig::new()
                .with_env()
                .init(),
        };

        debug!("Sui CLI version: {VERSION}");

        exit_main!(args.command.execute().await);

        Ok("SUCCESS".to_string())

    }

}