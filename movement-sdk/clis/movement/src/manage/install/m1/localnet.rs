use async_trait::async_trait;
use clap::{Subcommand, Parser};
use util::cli::Command;

#[derive(Debug, Parser)]
pub struct Localnet {
    
}

#[async_trait]
impl Command<String> for Localnet {

    async fn get_name(&self) -> String {
        "localnet".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {
        Ok("SUCCESS".to_string())
    }

}