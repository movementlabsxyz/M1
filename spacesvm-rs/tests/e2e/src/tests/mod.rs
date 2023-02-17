use std::{
    fs,
    path::Path,
    str::FromStr,
    thread,
    time::{Duration, Instant},
};

use avalanche_network_runner_sdk::{BlockchainSpec, Client, GlobalConfig, StartRequest};
use avalanche_types::{ids, subnet};
use spacesvm::{
    self,
    api::client::{claim_tx, get_or_create_pk, set_tx, Uri},
};

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

    let vm_id = subnet::vm_name_to_id("spacesvm").unwrap();

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
        // keep this in sync with "proto" crate
        // ref. https://github.com/ava-labs/avalanchego/blob/v1.9.2/version/constants.go#L15-L17
        let (exec_path, plugins_dir) =
            avalanche_installer::avalanchego::download(None, None, Some("v1.9.3".to_string()))
                .await
                .unwrap();
        avalanchego_exec_path = exec_path;
        plugins_dir
    };

    log::info!(
        "copying vm plugin {} to {}/{}",
        vm_plugin_path,
        plugins_dir,
        vm_id
    );
    fs::copy(
        &vm_plugin_path,
        Path::new(&plugins_dir).join(&vm_id.to_string()),
    )
    .unwrap();

    // write some random genesis file
    let genesis = spacesvm::genesis::Genesis {
        author: random_manager::string(5),
        welcome_message: random_manager::string(10),
    };
    let genesis_file_path = random_manager::tmp_path(10, None).unwrap();
    genesis.sync(&genesis_file_path).unwrap();

    log::info!(
        "starting {} with avalanchego {}, genesis file path {}",
        vm_id,
        avalanchego_exec_path,
        genesis_file_path,
    );
    let resp = cli
        .start(StartRequest {
            exec_path: avalanchego_exec_path,
            num_nodes: Some(5),
            plugin_dir: Some(plugins_dir),
            global_node_config: Some(
                serde_json::to_string(&GlobalConfig {
                    log_level: String::from("info"),
                })
                .unwrap(),
            ),
            blockchain_specs: vec![BlockchainSpec {
                vm_name: String::from("spacesvm"),
                genesis: genesis_file_path.to_string(),
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
            if ci.custom_chains.len() > 0 {
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
        if v.chain_name == "spacesvm" {
            blockchain_id = ids::Id::from_str(&v.chain_id).unwrap();
            break;
        }
    }

    log::info!("avalanchego RPC endpoints: {:?}", rpc_eps);

    let private_key = get_or_create_pk("/tmp/.spacesvm-cli-pk").expect("generate new private key");
    let chain_url = format!("{}/ext/bc/{}/public", rpc_eps[0], blockchain_id);
    let scli =
        spacesvm::api::client::Client::new(chain_url.parse::<Uri>().expect("valid endpoint"));
    scli.set_private_key(private_key).await;
    for ep in rpc_eps.iter() {
        let chain_url = format!("{}/ext/bc/{}/public", ep, blockchain_id)
            .parse::<Uri>()
            .expect("valid endpoint");
        scli.set_endpoint(chain_url).await;
        let resp = scli.ping().await.unwrap();
        log::info!("ping response from {}: {:?}", ep, resp);
        assert!(resp.success);

        thread::sleep(Duration::from_millis(300));
    }

    scli.set_endpoint(chain_url.parse::<Uri>().expect("valid endpoint"))
        .await;

    log::info!("decode claim tx request...");
    let resp = scli
        .decode_tx(claim_tx("test"))
        .await
        .expect("decodeTx success");
    log::info!("decode claim tx response from {}: {:?}", chain_url, resp);

    log::info!("issue claim tx request...");
    let resp = scli
        .issue_tx(&resp.typed_data)
        .await
        .expect("issue_tx success");
    log::info!("issue claim tx response from {}: {:?}", chain_url, resp);

    log::info!("decode set tx request...");
    let resp = scli
        .decode_tx(set_tx("test", "foo", "bar"))
        .await
        .expect("decodeTx success");
    log::info!("decode set tx response from {}: {:?}", chain_url, resp);

    log::info!("issue set tx request...");
    let resp = scli
        .issue_tx(&resp.typed_data)
        .await
        .expect("issue_tx success");
    log::info!("issue set tx response from {}: {:?}", chain_url, resp);

    log::info!("issue resolve request...");
    let resp = scli.resolve("test", "foo").await.expect("resolve success");
    log::info!("resolve response from {}: {:?}", chain_url, resp);
    assert_eq!(std::str::from_utf8(&resp.value).unwrap(), "bar");

    if crate::get_network_runner_enable_shutdown() {
        log::info!("shutdown is enabled... stopping...");
        let _resp = cli.stop().await.expect("failed stop");
        log::info!("successfully stopped network");
    } else {
        log::info!("skipped network shutdown...");
    }
}
