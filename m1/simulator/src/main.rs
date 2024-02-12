use clap::{Parser, Subcommand};

type NodeId = u64;

#[derive(Debug, Parser)]
#[clap(name = "forc index", about = "M1 network simulator", version = "0.1")]
pub struct Cli {
    /// The command to run
    #[clap(subcommand)]
    pub command: SubCommands,
}

/// Start the simulator 
#[derive(Debug, Parser)]
pub struct StartCommand {
    /// The number of validators for the network
    #[clap(long, default_value = "5", help = "The number of validators for the network.")]
    pub validators: u64,

    /// Sets if the validators join the network at once, or in a staggered way
    #[clap(long, default_value = "false", help = "Sets if the validators join the network at once, or in a staggered way.")]
    pub staggered: bool,

    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool
}

/// Partition the network
#[derive(Debug, Parser)]
pub struct PartitionCommand {
    /// The percentage of validators that will be partitioned
    #[clap(long, default_value = "5", help = "The percentage of validators that will be in a partitioned state")]
    pub amount: u8, 

    /// Sets if the validators become paritioned at once or in a staggered way
    #[clap(long, default_value = "false", help = "Sets if the validators become partitioned at once or in a staggered way.")]
    pub staggered: bool,

    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool
}

#[derive(Debug, Parser)]
pub struct ReconnectCommand {
    /// The nodes to reconnect by `NodeId`
    pub nodes: Vec<NodeId>,

    /// Sets if the validators rejoin the network together or in a staggered way
    #[clap(long, default_value = "false")]
    pub staggered: bool,

    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool
}

#[derive(Debug, Parser)]
pub struct HealthCommand {
    /// Verbose ouput
     #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool
}

#[derive(Debug, Subcommand)]
pub enum SubCommands{
    /// Starts the network with a number of validators
    Start(StartCommand),
    /// Simulates a network partition. 
    Partition(PartitionCommand),
    /// Reconnects the validators after they have become partitioned
    Reconnect(ReconnectCommand),
    /// Output the overall network and consensus health
    Health(HealthCommand),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        SubCommands::Start(opts) => start_network(opts).await,
        SubCommands::Partition(opts) => partition_network(opts).await,
        SubCommands::Reconnect(opts) => reconnect_validators(opts).await,
        SubCommands::Health(opts) => network_health(opts).await,
    }
}

async fn start_network(opts: StartCommand) {
    simulator::init_m1_network().await;

}

async fn partition_network(opts: PartitionCommand) {
    todo!()
}

async fn reconnect_validators(opts: ReconnectCommand) {
    todo!()

}

async fn network_health(opts: HealthCommand) {
    todo!()
}