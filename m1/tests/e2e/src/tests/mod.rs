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

const AVALANCHEGO_VERSION: &str = "v1.10.9";

// todo: extracted from genesis method
// todo: really we should use a genesis once more 
pub fn sync_genesis(byte_string : &str, file_path: &str) -> io::Result<()> {
    log::info!("syncing genesis to '{}'", file_path);

    let path = Path::new(file_path);
    let parent_dir = path.parent().expect("Invalid path");
    fs::create_dir_all(parent_dir)?;

    let d = byte_string.as_bytes();

    let mut f = File::create(file_path)?;
    f.write_all(&d)?;

    Ok(())
}

#[tokio::test]
async fn e2e() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let (ep, is_set) = crate::get_network_runner_grpc_endpoint();
    assert!(is_set);

    let cli = Client::new(&ep).await;

    log::info!("ping...");
    let resp = cli.ping().await.expect("failed ping");
    log::info!("network-runner is running (ping response {:?})", resp);

    let (vm_plugin_path, exists) = crate::get_vm_plugin_path();
    log::info!("Vm Plugin path: {vm_plugin_path}");
    assert!(exists);
    assert!(Path::new(&vm_plugin_path).exists());

    let vm_id = Path::new(&vm_plugin_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    // ! for now, we hardcode the id to be subnet for orchestration
    let vm_id = subnet::vm_name_to_id("subnet").unwrap();

    let (mut avalanchego_exec_path, _) = crate::get_avalanchego_path();
    let plugins_dir = if !avalanchego_exec_path.is_empty() {
        let parent_dir = Path::new(&avalanchego_exec_path)
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
        avalanchego_exec_path = exec_path;
        avalanche_installer::avalanchego::get_plugin_dir(&avalanchego_exec_path)
    };

    log::info!(
        "copying vm plugin {} to {}/{}",
        vm_plugin_path,
        plugins_dir,
        vm_id
    );

    fs::create_dir(&plugins_dir).unwrap();
    fs::copy(
        &vm_plugin_path,
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
        &avalanchego_exec_path,
        genesis_file_path,
    );
    let resp = cli
        .start(StartRequest {
            exec_path: avalanchego_exec_path,
            num_nodes: Some(5),
            plugin_dir: plugins_dir,
            global_node_config: Some(
                serde_json::to_string(&GlobalConfig {
                    log_level: String::from("info"),
                })
                .unwrap(),
            ),
            blockchain_specs: vec![BlockchainSpec {
                vm_name: String::from("subnet"),
                genesis: genesis_file_path.to_string(),
                blockchain_alias : String::from("subnet"), // todo: this doesn't always work oddly enough, need to debug
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
    let timeout = Duration::from_secs(
        std::env::var("SUBNET_TIMEOUT")
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u64>()
            .unwrap(),
    );
    log::info!("sleeping for {} seconds", timeout.as_secs());
    thread::sleep(timeout);

}
