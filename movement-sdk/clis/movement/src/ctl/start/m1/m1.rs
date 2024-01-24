use clap::Subcommand;
use util::cli::Command;
use super::testnet::Testnet;

#[derive(Subcommand, Debug)]
#[clap(
    rename_all = "kebab-case",
    about = "Start an M1 service"
)]
pub enum M1 {
    Testnet(Testnet)
}

#[async_trait::async_trait]
impl Command<String> for M1 {

    async fn get_name(&self) -> String {
        "start".to_string()
    }

    async fn execute(self) -> Result<String, anyhow::Error> {

        match self {
            M1::Testnet(testnet) => testnet.execute().await?
        };

        Ok("SUCCESS".to_string())
    }

}