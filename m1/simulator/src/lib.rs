use anyhow::{anyhow, Context, Result};
use core::time;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    str::FromStr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use avalanche_network_runner_sdk::{
    rpcpb::{
        AddPermissionlessValidatorRequest, AddSubnetValidatorsRequest, CustomChainInfo, PermissionlessStakerSpec, RemoveSubnetValidatorsRequest, SubnetValidatorsSpec, VmidRequest, VmidResponse
    },
    AddNodeRequest, BlockchainSpec, Client, GlobalConfig, RemoveNodeRequest, StartRequest,
};
use avalanche_types::{
    ids::{self, Id as VmId},
    jsonrpc::client::info as avalanche_sdk_info,
    subnet::{self, vm_name_to_id},
};
use commands::*;
use tonic::transport::Channel;

pub mod commands;

const AVALANCHEGO_VERSION: &str = "v1.10.12";
pub const LOCAL_GRPC_ENDPOINT: &str = "http://127.0.0.1:12342";
const VM_NAME: &str = "subnet";

/// The Simulator is used to run commands on the avalanche-go-network-runner.
/// It can be used across multiple threads and is thread safe.
pub struct Simulator {
    /// The network runner client
    pub cli: Arc<Client<Channel>>,
    /// The command to run
    pub command: SubCommands,
    /// The path to the avalanchego binary, must be version no higher than 1.10.12
    /// higher versions use a VM plugin version higher that `28`, which is used by `subnet`
    pub avalanchego_path: String,
    /// The path to the VM plugin
    pub vm_plugin_path: String,
    /// The subnet ID created by the network runner,
    /// this is not the same value as the VM ID.
    pub subnet_id: Option<String>,
}

impl Simulator {
    pub async fn new(command: SubCommands) -> Result<Self> {
        let cli = Client::new(LOCAL_GRPC_ENDPOINT).await;
        Ok(Self {
            cli: Arc::new(cli),
            command,
            avalanchego_path: get_avalanchego_path()?,
            vm_plugin_path: get_vm_plugin_path()?,
            subnet_id: None,
        })
    }

    pub async fn dispatch(&mut self) -> Result<()> {
        match &self.command {
            SubCommands::Start(cmd) => self.start_network(cmd.clone()).await?,
            SubCommands::AddNode(cmd) => self.add_node(cmd.clone()).await?,
            SubCommands::RemoveNode(cmd) => self.remove_node(cmd.clone()).await?,
            SubCommands::AddValidator(cmd) => self.add_validator(cmd.clone()).await?,
            SubCommands::RemoveValidator(cmd) => self.remove_validator(cmd.clone()).await?,
            SubCommands::Partition(cmd) => self.partition_network(cmd.clone()).await?,
            SubCommands::Reconnect(cmd) => self.reconnect_validators(cmd.clone()).await?,
            SubCommands::Health(cmd) => self.network_health(cmd.clone()).await?,
        }
        Ok(())
    }

    pub async fn start_network(&mut self, cmd: StartCommand) -> Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(if cmd.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();
        log::debug!("Running command: {:?}", cmd);

        let vm_id = Path::new(&self.vm_plugin_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let vm_id = subnet::vm_name_to_id("subnet").unwrap();

        let plugins_dir = if !&self.avalanchego_path.is_empty() {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").context("No manifest dir found")?;
            let workspace_dir = Path::new(&manifest_dir)
                .parent()
                .context("No parent dir found")?;
            workspace_dir
                .join("plugins")
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string()
        } else {
            // Don't think this block will ever get hit in the current state
            let exec_path = avalanche_installer::avalanchego::github::download(
                None,
                None,
                Some(AVALANCHEGO_VERSION.to_string()),
            )
            .await
            .unwrap();
            self.avalanchego_path = exec_path.clone();
            avalanche_installer::avalanchego::get_plugin_dir(&self.avalanchego_path)
        };

        log::info!(
            "copying vm plugin {} to {}/{}",
            self.vm_plugin_path,
            plugins_dir,
            vm_id
        );

        //fs::create_dir(&plugins_dir)?;
        fs::copy(
            &self.vm_plugin_path,
            Path::new(&plugins_dir).join(vm_id.to_string()),
        )?;

        // write some random genesis file
        let genesis = random_manager::secure_string(10);

        let genesis_file_path = random_manager::tmp_path(10, None).unwrap();
        sync_genesis(genesis.as_ref(), &genesis_file_path).unwrap();

        log::info!(
            "starting {} with avalanchego {}, genesis file path {}",
            vm_id,
            &self.avalanchego_path,
            genesis_file_path,
        );
        log::debug!(
            "plugins dir: {}, global node config: {:?}",
            plugins_dir,
            serde_json::to_string(&GlobalConfig {
                log_level: String::from("info"),
            })
        );
        let resp = self
            .cli
            .start(StartRequest {
                exec_path: self.avalanchego_path.clone(),
                num_nodes: Some(5),
                plugin_dir: plugins_dir,
                global_node_config: Some(
                    serde_json::to_string(&GlobalConfig {
                        log_level: String::from("info"),
                    })
                    .unwrap(),
                ),
                blockchain_specs: vec![BlockchainSpec {
                    vm_name: String::from(VM_NAME),
                    genesis: genesis_file_path.to_string(),
                    //blockchain_alias : String::from("subnet"), // todo: this doesn't always work oddly enough, need to debug
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await?;
        log::info!(
            "started avalanchego cluster with network-runner: {:?}",
            resp
        );

        // enough time for network-runner to get ready
        thread::sleep(Duration::from_secs(20));

        log::info!("checking cluster healthiness...");
        let mut ready = false;

        let timeout = Duration::from_secs(300);
        let interval = Duration::from_secs(15);
        let start = Instant::now();
        let mut cnt: u128 = 0;
        loop {
            let elapsed = start.elapsed();
            if elapsed.gt(&timeout) {
                break;
            }

            let itv = {
                if cnt == 0 {
                    // first poll with no wait
                    Duration::from_secs(1)
                } else {
                    interval
                }
            };
            thread::sleep(itv);

            ready = {
                match self.cli.health().await {
                    Ok(_) => {
                        log::info!("healthy now!");
                        true
                    }
                    Err(e) => {
                        log::warn!("not healthy yet {}", e);
                        false
                    }
                }
            };
            if ready {
                break;
            }

            cnt += 1;
        }
        assert!(ready);

        log::info!("checking status...");
        let mut status = self.cli.status().await.expect("failed status");
        loop {
            let elapsed = start.elapsed();
            if elapsed.gt(&timeout) {
                break;
            }

            if let Some(ci) = &status.cluster_info {
                if !ci.custom_chains.is_empty() {
                    break;
                }
            }

            log::info!("retrying checking status...");
            thread::sleep(interval);
            status = self.cli.status().await.expect("failed status");
        }

        assert!(status.cluster_info.is_some());
        let cluster_info = status.cluster_info.unwrap();
        let mut rpc_eps: Vec<String> = Vec::new();
        for (node_name, iv) in cluster_info.node_infos.into_iter() {
            log::info!("{}: {}", node_name, iv.uri);
            rpc_eps.push(iv.uri.clone());
        }
        let mut blockchain_id = ids::Id::empty();
        for (k, v) in cluster_info.custom_chains.iter() {
            log::info!("custom chain info: {}={:?}", k, v);
            if v.chain_name == "subnet" {
                blockchain_id = ids::Id::from_str(&v.chain_id).unwrap();
                break;
            }
        }
        log::info!("avalanchego RPC endpoints: {:?}", rpc_eps);
        let resp = avalanche_sdk_info::get_network_id(&rpc_eps[0])
            .await
            .unwrap();
        let network_id = resp.result.unwrap().network_id;
        log::info!("network Id: {}", network_id);

        Ok(())
    }

    pub async fn partition_network(&self, cmd: PartitionCommand) -> Result<()> {
        Ok(())
    }

    pub async fn reconnect_validators(&self, cmd: ReconnectCommand) -> Result<()> {
        Ok(())
    }

    pub async fn network_health(&self, cmd: HealthCommand) -> Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(if cmd.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();
        log::debug!("Running command: {:?}", cmd);
        let resp = self.cli.health().await?;
        log::info!("network health: {:?}", resp);
        Ok(())
    }

    pub async fn add_node(&self, cmd: AddNodeCommand) -> Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(if cmd.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();
        log::debug!("Running command: {:?}", cmd.clone());
        let name = match cmd.name {
            Some(n) => format!("node-{}", n),
            None => format!("node-{}", random_manager::secure_string(5)),
        };
        let resp = self
            .cli
            .add_node(AddNodeRequest {
                name,
                exec_path: self.avalanchego_path.clone(),
                node_config: None,
                ..Default::default()
            })
            .await?;
        log::info!("added node: {:?}", resp);
        Ok(())
    }

    pub async fn remove_node(&self, cmd: RemoveNodeCommand) -> Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(if cmd.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();
        log::debug!("Running command: {:?}", cmd);
        let resp = self
            .cli
            .remove_node(RemoveNodeRequest {
                name: cmd.name,
                ..Default::default()
            })
            .await?;
        log::info!("removed node: {:?}", resp);
        Ok(())
    }

    pub async fn add_validator(&self, cmd: AddValidatorCommand) -> Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(if cmd.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();

        let resp = self.cli.health().await?;
        let custom_chains: HashMap<String, CustomChainInfo> = resp
            .cluster_info
            .expect("no cluster info found")
            .custom_chains
            .clone();

        let mut subnet_id = String::new();

        // This is a bit brittle, but there is only one custom chain in the HashMap:
        // the subnet. If more subets wanted to be tested on the simulator this wouldn't work.
        // Create tracking issue for this. The problem is I can't get the blockchain ID as the key
        // lookup for custom_chains.
        for (i, chain) in custom_chains.into_iter().enumerate() {
            if i == 0 {
                subnet_id = chain.1.subnet_id;
            }
        }

        let resp = self
            .cli
            .add_validator(AddSubnetValidatorsRequest {
                validator_spec: vec![{
                    SubnetValidatorsSpec {
                        subnet_id,
                        node_names: vec![cmd.name],
                    }
                }],
            })
            .await?;
        log::info!("added validator: {:?}", resp);
        Ok(())
    }

    pub async fn remove_validator(&self, cmd: RemoveValidatorCommand) -> Result<()> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(if cmd.verbose {
                "debug"
            } else {
                "info"
            }),
        )
        .init();
        log::debug!("Running command: {:?}", cmd);
        let resp = self
            .cli
            .remove_validator(RemoveSubnetValidatorsRequest {
                ..Default::default()
            })
            .await?;
        log::info!("removed validator: {:?}", resp);
        Ok(())
    }
}

#[must_use]
pub fn get_network_runner_grpc_endpoint() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_GRPC_ENDPOINT") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}

#[must_use]
pub fn get_network_runner_enable_shutdown() -> bool {
    matches!(std::env::var("NETWORK_RUNNER_ENABLE_SHUTDOWN"), Ok(_))
}

#[must_use]
pub fn get_avalanchego_path() -> Result<String, anyhow::Error> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").context("No manifest dir found")?;
    let manifest_path = Path::new(&manifest_dir);

    //Navigate two levels up from the Cargo manifest directory ../../
    let avalanchego_path = manifest_path
        .parent()
        .context("No parent dirctory found")?
        .parent()
        .context("No parent directory found")?
        .parent()
        .context("No parent directory found")?
        .join("avalanchego")
        .join("build")
        .join("avalanchego");

    if !avalanchego_path.exists() {
        log::debug!("avalanchego path: {:?}", avalanchego_path);
        return Err(anyhow!(
            "
                    avalanchego binary not in expected path. 
                    Install the binary at the expected path {:?}",
            avalanchego_path
        ));
    }

    let path_buf = avalanchego_path
        .to_str()
        .context("Failed to convert path to string")?;
    log::debug!("avalanchego path: {}", path_buf);
    Ok(path_buf.to_string())
}

#[must_use]
pub fn get_vm_plugin_path() -> Result<String, anyhow::Error> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").context("No manifest dir found")?;
    let manifest_path = Path::new(&manifest_dir);

    // Construct the path to the binary with ./target/debug/subnet
    let subnet_path = manifest_path
        .parent()
        .context("Could not find the parent dir")?
        .join("target")
        .join("debug")
        .join("subnet");
    if !subnet_path.exists() {
        log::debug!("vm plugin path: {:?}", subnet_path);
        return Err(anyhow!(
            "
                    vm plugin not in expected path. 
                    Install the plugin at the expected path {:?}",
            subnet_path
        ));
    }

    let path_buf = subnet_path
        .to_str()
        .context("Failed to convert path to string")?;
    log::debug!("vm plugin path: {}", path_buf);
    Ok(path_buf.to_string())
}

// todo: extracted from genesis method
// todo: really we should use a genesis once more
pub fn sync_genesis(byte_string: &str, file_path: &str) -> io::Result<()> {
    log::info!("syncing genesis to '{}'", file_path);

    let path = Path::new(file_path);
    let parent_dir = path.parent().expect("Invalid path");
    fs::create_dir_all(parent_dir)?;

    let d = byte_string.as_bytes();

    let mut f = File::create(file_path)?;
    f.write_all(&d)?;

    Ok(())
}
