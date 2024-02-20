use core::time;
use std::{
    io,
    fs::{self, File},
    path::Path,
    str::FromStr,
    thread,
    time::{Duration, Instant}, io::Write,
};

use avalanche_network_runner_sdk::{BlockchainSpec, Client, GlobalConfig, StartRequest};
use avalanche_types::{ids, jsonrpc::client::info as avalanche_sdk_info, subnet};
use simulator::{get_network_runner_grpc_endpoint, get_network_runner_enable_shutdown,
get_avalanchego_path, get_vm_plugin_path, init_m1_network};

const AVALANCHEGO_VERSION: &str = "v1.3.7";

#[tokio::test] 
async fn e2e() {
    init_m1_network().await
}
