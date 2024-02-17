use clap::{Parser, Subcommand};
use env_logger::{Builder, Env};
use simulator::{get_avalanchego_path, get_vm_plugin_path, Network};

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
    #[clap(
        long,
        default_value = "5",
        help = "The number of validators for the network."
    )]
    pub validators: u64,

    /// Sets if the validators join the network at once, or in a staggered way
    #[clap(
        long,
        default_value = "false",
        help = "Sets if the validators join the network at once, or in a staggered way."
    )]
    pub staggered: bool,

    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,

    /// Run simuilator against the local network
    #[clap(long, help = "Run simuilator against the local network.")]
    pub local: bool,

    /// The GRPC endpoint of the network runner to connect to
    #[clap(long, help = "The GRPC endpoint of the network runner to connect to.")]
    pub grpc_endpoint: Option<String>,
}

/// Partition the network
#[derive(Debug, Parser)]
pub struct PartitionCommand {
    /// The percentage of validators that will be partitioned
    #[clap(
        long,
        default_value = "5",
        help = "The percentage of validators that will be in a partitioned state"
    )]
    pub amount: u8,

    /// Sets if the validators become paritioned at once or in a staggered way
    #[clap(
        long,
        default_value = "false",
        help = "Sets if the validators become partitioned at once or in a staggered way."
    )]
    pub staggered: bool,

    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,
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
    pub verbose: bool,
}

#[derive(Debug, Parser)]
pub struct HealthCommand {
    /// Verbose ouput
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
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
async fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();
    match cli.command {
        SubCommands::Start(opts) => start_network(opts).await?,
        SubCommands::Partition(opts) => partition_network(opts).await?,
        SubCommands::Reconnect(opts) => reconnect_validators(opts).await?,
        SubCommands::Health(opts) => network_health(opts).await?,
    }
    Ok(())
}

async fn start_network(opts: StartCommand) -> Result<(), anyhow::Error> {
    // Set log level based on verbosity
    Builder::from_env(Env::default().default_filter_or(if opts.verbose {
        "debug"
    } else {
        "info"
    }))
    .init();

    let avalanche_path = get_avalanchego_path(opts.local)?;
    let vm_path = get_vm_plugin_path()?;
    let mut net = Network::new(
        opts.local,
        opts.grpc_endpoint,
        opts.staggered,
        avalanche_path,
        vm_path,
    )?;

    net.init_m1_network().await?;
    Ok(())
}

async fn partition_network(opts: PartitionCommand) -> Result<(), anyhow::Error> {
    Ok(())
}

async fn reconnect_validators(opts: ReconnectCommand) -> Result<(), anyhow::Error> {
    Ok(())
}

async fn network_health(opts: HealthCommand) -> Result<(), anyhow::Error> {
    Ok(())
}
