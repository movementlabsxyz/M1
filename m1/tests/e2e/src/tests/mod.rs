use avalanche_network_runner_sdk::{BlockchainSpec, Client, GlobalConfig, StartRequest};
use avalanche_types::{ids, jsonrpc::client::info as avalanche_sdk_info, subnet};
use simulator::{
    commands::{StartCommand, SubCommands},
    Simulator,
};

const AVALANCHEGO_VERSION: &str = "v1.3.7";

#[tokio::test]
async fn e2e() {
    let cmd = StartCommand {
        nodes: 5,
        staggered: false,
        verbose: false,
        grpc_endpoint: None,
    };
    let mut simulator = Simulator::new(SubCommands::Start(cmd))
        .await
        .expect("Failed to create simulator");
    simulator.exec().await.expect("Failed to execute simulator");
}
