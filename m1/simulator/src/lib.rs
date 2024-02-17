use core::time;
use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};

use avalanche_network_runner_sdk::{BlockchainSpec, Client, GlobalConfig, StartRequest};
use avalanche_types::{
    ids::{self, Id as VmId},
    jsonrpc::client::info as avalanche_sdk_info,
    subnet::{self, vm_name_to_id},
};

const LOCAL_GRPC_ENDPOINT: &str = "http://127.0.0.1:12342";
const VM_NAME: &str = "subnet";

/// Network configuration
pub struct Network {
    /// The GRPC endpoint of the network runner to connect to
    pub grpc_endpoint: Option<String>,
    /// Sets if the validators join the network at once, or in a staggered way
    pub enable_shutdown: bool,
    /// The path to the avalanchego binary
    pub avalanchego_path: String,
    /// The path to the VM plugin
    pub vm_plugin_path: String,
    /// VM name, this is hardcoded for now
    pub vm_name: VmId,
}

impl Network {
    /// Create a new network configuration

    pub fn new(
        is_local: bool,
        grpc_endpoint: Option<String>,
        enable_shutdown: bool,
        avalanchego_path: String,
        vm_plugin_path: String,
    ) -> Result<Self> {
        let grpc_endpoint = match is_local {
            true => Some(LOCAL_GRPC_ENDPOINT.to_string()),
            false => {
                if let Some(endpoint) = grpc_endpoint {
                    Some(endpoint)
                } else {
                    return Err(anyhow!("GRPC endpoint not provided"));
                }
            }
        };
        Ok(Network {
            grpc_endpoint,
            enable_shutdown,
            avalanchego_path,
            vm_plugin_path,
            vm_name: subnet::vm_name_to_id(VM_NAME)?,
        })
    }

    pub async fn init_m1_network(&mut self) -> Result<(), anyhow::Error> {
        let grpc = match self.grpc_endpoint.clone() {
            Some(grpc) => grpc,
            None => {
                return Err(anyhow!("GRPC endpoint not provided"));
            }
        };
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Info)
            .is_test(true)
            .try_init();

        let cli = Client::new(&grpc).await;

        log::info!("ping...");
        let resp = cli.ping().await.expect("failed ping");
        log::info!("network-runner is running (ping response {:?})", resp);

        let vm_id = Path::new(&self.vm_plugin_path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let plugins_dir = if !&self.avalanchego_path.is_empty() {
            let parent_dir = Path::new(&self.avalanchego_path)
                .parent()
                .expect("unexpected None parent");
            parent_dir
                .join("plugins")
                .as_os_str()
                .to_str()
                .unwrap()
                .to_string()
        } else {
            let exec_path = avalanche_installer::avalanchego::github::download(
                None,
                None,
                Some(AVALANCHEGO_VERSION.to_string()),
            )
            .await
            .unwrap();
            self.avalanchego_path = exec_path;
            avalanche_installer::avalanchego::get_plugin_dir(&self.avalanchego_path)
        };

        log::info!(
            "copying vm plugin {} to {}/{}",
            self.vm_plugin_path,
            plugins_dir,
            vm_id
        );

        fs::create_dir(&plugins_dir).unwrap();
        fs::copy(
            &self.vm_plugin_path,
            Path::new(&plugins_dir).join(vm_id.to_string()),
        )
        .unwrap();

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
        let resp = cli
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
                    // blockchain_alias : String::from("subnet"), // todo: this doesn't always work oddly enough, need to debug
                    ..Default::default()
                }],
                ..Default::default()
            })
            .await
            .expect("failed start");
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
                match cli.health().await {
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
        let mut status = cli.status().await.expect("failed status");
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
            status = cli.status().await.expect("failed status");
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

        // keep alive by sleeping for duration provided by SUBNET_TIMEOUT environment variable
        // use sensible default

        let val = std::env::var("SUBNET_TIMEOUT")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<i64>()
            .unwrap();

        log::info!("sleeping for {} seconds", timeout.as_secs());
        if val < 0 {
            // run forever
            loop {
                thread::sleep(Duration::from_secs(1000));
            }
        } else {
            let timeout = Duration::from_secs(val as u64);
            thread::sleep(timeout);
        }
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
pub fn get_avalanchego_path(is_local: bool) -> Result<String, anyhow::Error> {
    match is_local {
        true => {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").context("No manifest dir found")?;
            let manifest_path = Path::new(&manifest_dir);

            //Navigate two levels up from the Cargo manifest directory ../../
            let root = manifest_path
                .parent()
                .context("No parent dirctory found")?
                .parent()
                .context("No parent directory found")?
                .parent()
                .context("No parent directory found")?
                .parent()
                .context("No parent directory found")?;

            let avalanchego_path = root.join(".avalanchego");
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
        false => match std::env::var("AVALANCHEGO_PATH") {
            Ok(s) => Ok(s),
            _ => Err(anyhow!("AVALANCHEGO_PATH not provided")),
        },
    }
}

#[must_use]
pub fn get_vm_plugin_path() -> Result<String, anyhow::Error> {
    match std::env::var("VM_PLUGIN_PATH") {
        Ok(s) => Ok(s),
        _ => Err(anyhow!("VM_PLUGIN_PATH not provided")),
    }
}

const AVALANCHEGO_VERSION: &str = "v1.10.9";

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

pub async fn init_m1_network() {}
