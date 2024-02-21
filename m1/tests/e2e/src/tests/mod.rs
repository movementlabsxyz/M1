use simulator::{
    commands::{StartCommand, SubCommands},
    Simulator,
};

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
    simulator
        .exec(cmd.verbose)
        .await
        .expect("Failed to execute simulator");
}
