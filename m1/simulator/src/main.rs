use clap::Parser;
use simulator::{commands::Cli, Simulator};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    let mut simulator = Simulator::new(cli.command).await?;
    simulator.exec(cli.verbose).await?;
    Ok(())
}
