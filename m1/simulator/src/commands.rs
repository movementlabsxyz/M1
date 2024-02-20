use clap::{Parser, Subcommand};

#[derive(Debug, Parser, Clone)]
#[clap(name = "forc index", about = "M1 network simulator", version = "0.1")]
pub struct Cli {
    /// The command to run
    #[clap(subcommand)]
    pub command: SubCommands,
}

/// Start the simulator
#[derive(Debug, Parser, Clone)]
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

    /// The GRPC endpoint of the network runner to connect to
    #[clap(long, help = "The GRPC endpoint of the network runner to connect to.")]
    pub grpc_endpoint: Option<String>,
}

/// Partition the network
#[derive(Debug, Parser, Clone)]
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

#[derive(Debug, Parser, Clone)]
pub struct ReconnectCommand {
    /// The nodes to reconnect by `NodeId`
    pub nodes: Vec<u64>,

    /// Sets if the validators rejoin the network together or in a staggered way
    #[clap(long, default_value = "false")]
    pub staggered: bool,

    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,
}

/// Add a node to the network
#[derive(Debug, Parser, Clone)]
pub struct AddNodeCommand {
    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,

    /// The name of the node to add
    #[clap(long, help = "The name of the node to add.")]
    pub name: Option<String>,
}

#[derive(Debug, Parser, Clone)]
pub struct RemoveNodeCommand {
    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,

    /// The name of the node to remove
    #[clap(long, help = "The name of the node to remove.")]
    pub name: String,
}

#[derive(Debug, Parser, Clone)]
pub struct AddValidatorCommand {
    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,

    /// The name of the validator to add
    #[clap(long, help = "The name of the validator to add.")]
    pub name: String,
}

#[derive(Debug, Parser, Clone)]
pub struct RemoveValidatorCommand {
    /// Verbose output
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,

    /// The name of the validator to remove
    #[clap(long, help = "The name of the validator to remove.")]
    pub name: String,
}

#[derive(Debug, Parser, Clone)]
pub struct HealthCommand {
    /// Verbose ouput
    #[clap(short, long, help = "Verbose output.")]
    pub verbose: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum SubCommands {
    /// Starts the network with a number of validators
    Start(StartCommand),
    /// Adds a node to the network
    AddNode(AddNodeCommand),
    /// Removes a node from the network
    RemoveNode(RemoveNodeCommand),
    /// Adds a validator to the network
    AddValidator(AddValidatorCommand),
    /// Removes a validator from the network
    RemoveValidator(RemoveValidatorCommand),
    /// Simulates a network partition.
    Partition(PartitionCommand),
    /// Reconnects the validators after they have become partitioned
    Reconnect(ReconnectCommand),
    /// Output the overall network and consensus health
    Health(HealthCommand),
}
