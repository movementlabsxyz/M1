use std::io::Result;

use avalanche_types::subnet;
use clap::{crate_version, Arg, Command};
use log::info;

use spacesvm::{genesis, vm};

pub const APP_NAME: &str = "spacesvm-rs";

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new(APP_NAME)
        .version(crate_version!())
        .about("key-value VM for Avalanche in Rust")
        .subcommands(vec![command_genesis()])
        .get_matches();

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    if let Some(("genesis", sub_matches)) = matches.subcommand() {
        let author = sub_matches
            .get_one::<String>("AUTHOR")
            .map(String::as_str)
            .unwrap_or("");
        let msg = sub_matches
            .get_one::<String>("WELCOME_MESSAGE")
            .map(String::as_str)
            .unwrap_or("");
        let p = sub_matches
            .get_one::<String>("GENESIS_FILE_PATH")
            .map(String::as_str)
            .unwrap_or("");
        execute_genesis(author, msg, p).unwrap();
        return Ok(());
    }

    // Initialize broadcast stop channel used to terminate gRPC servers during shutdown.
    let (stop_ch_tx, stop_ch_rx): (
        tokio::sync::broadcast::Sender<()>,
        tokio::sync::broadcast::Receiver<()>,
    ) = tokio::sync::broadcast::channel(1);

    info!("starting spacesvm-rs");
    let vm_server = subnet::rpc::vm::server::Server::new(vm::ChainVm::new(), stop_ch_tx);

    subnet::rpc::plugin::serve(vm_server, stop_ch_rx)
        .await
        .expect("failed to start gRPC server");

    Ok(())
}

pub fn command_genesis() -> Command {
    Command::new("genesis")
        .about("Generates the genesis file")
        .arg(
            Arg::new("AUTHOR")
                .long("author")
                .short('a')
                .help("author of the genesis")
                .required(true)
                .default_value("subnet creator"),
        )
        .arg(
            Arg::new("WELCOME_MESSAGE")
                .long("welcome-message")
                .short('m')
                .help("message field in genesis")
                .required(true)
                .default_value("hi"),
        )
        .arg(
            Arg::new("GENESIS_FILE_PATH")
                .long("genesis-file-path")
                .short('p')
                .help("file path to save genesis file")
                .required(true),
        )
}

pub fn execute_genesis(author: &str, msg: &str, p: &str) -> Result<()> {
    let g = genesis::Genesis {
        author: String::from(author),
        welcome_message: String::from(msg),
    };
    g.sync(p)
}
