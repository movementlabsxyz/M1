pub mod client;

use avalanche_types::{ids, subnet::rpc::utils};
use jsonrpc_core::Params;
use spacesvm::{
    api::{client::claim_tx, DecodeTxArgs},
    vm::{self, PUBLIC_API_ENDPOINT},
};
use std::io::{Error, ErrorKind};
use tokio::{
    sync::{
        broadcast::{Receiver, Sender},
        mpsc,
    },
    time::{sleep, Duration},
};
use tonic::transport::Channel;

#[tokio::test]
async fn test_api() {
    // init logger
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    // setup stop channel for grpc services.
    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = tokio::sync::broadcast::channel(1);
    let vm_server =
        avalanche_types::subnet::rpc::vm::server::Server::new(vm::ChainVm::new(), stop_ch_tx);

    // start Vm service
    let vm_addr = utils::new_socket_addr();
    tokio::spawn(async move {
        avalanche_types::subnet::rpc::plugin::serve_with_address(vm_server, vm_addr, stop_ch_rx)
            .await
            .expect("failed to start gRPC server");
    });
    log::info!("started subnet vm");

    // wait for server to start
    sleep(Duration::from_millis(100)).await;

    // create gRPC client for Vm client.
    let client_conn = Channel::builder(format!("http://{}", vm_addr).parse().unwrap())
        .connect()
        .await
        .unwrap();

    let mut vm_client = crate::vm::client::Client::new(client_conn);

    let mut versioned_dbs: Vec<
        avalanche_types::subnet::rpc::database::manager::versioned_database::VersionedDatabase,
    > = Vec::with_capacity(1);
    versioned_dbs.push(
        avalanche_types::subnet::rpc::database::manager::versioned_database::VersionedDatabase::new(
            avalanche_types::subnet::rpc::database::memdb::Database::new(),
            semver::Version::parse("0.0.7").unwrap(),
        ),
    );

    let db_manager =
        avalanche_types::subnet::rpc::database::manager::DatabaseManager::new_from_databases(
            Vec::new(),
        );
    let app_sender = MockAppSender::new();
    let (tx_engine, mut rx_engine): (
        mpsc::Sender<avalanche_types::subnet::rpc::common::message::Message>,
        mpsc::Receiver<avalanche_types::subnet::rpc::common::message::Message>,
    ) = mpsc::channel(1);

    tokio::spawn(async move {
        while let Some(msg) = rx_engine.recv().await {
            log::info!("engine received message: {:?}", msg);
        }
    });

    let genesis_bytes =
        "{\"author\":\"subnet creator\",\"welcome_message\":\"Hello from Rust VM!\"}".as_bytes();

    let resp = vm_client
        .initialize(
            None,
            db_manager,
            genesis_bytes,
            &[],
            &[],
            tx_engine,
            &[()],
            app_sender,
        )
        .await;

    assert!(resp.is_ok());

    // call create_handlers.
    let resp = vm_client.create_handlers().await;
    assert!(resp.is_ok());

    let handlers = resp.unwrap();

    // get the "/public" handler we assume it exists because it was created during initialize.
    let handler = handlers.get(PUBLIC_API_ENDPOINT).unwrap();

    let http_addr = handler.clone().server_addr.as_ref().unwrap();

    // create client for http service which was started during create_handlers RPC.
    let client_conn = Channel::builder(format!("http://{}", http_addr).parse().unwrap())
        .connect()
        .await
        .unwrap();

    let client = spacesvm::api::client::Client::new(http::Uri::from_static("http://test.url"));

    // ping
    let (_id, json_str) = client
        .raw_request("ping", &Params::None)
        .await
        .expect("raw_request success");
    let req = http::request::Builder::new()
        .body(json_str.as_bytes().to_vec())
        .unwrap();

    // pass the http request to the serve_http_simple RPC. this same process
    // takes place when the avalanchego router passes a request to the
    // subnet process. this process also simulates a raw JSON request from
    // curl or postman.
    log::info!("sending http request over grpc");
    let mut http_client = avalanche_types::subnet::rpc::http::client::Client::new(client_conn);
    let resp = http_client.serve_http_simple(req).await.map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("failed to initialize vm: {:?}", e),
        )
    });

    assert!(resp.is_ok());
    let resp = resp.unwrap();
    let body = std::str::from_utf8(&resp.body()).unwrap();
    log::info!("ping response {}", body);

    let tx_data = claim_tx("test_claim");
    let arg_value = serde_json::to_value(&DecodeTxArgs { tx_data }).unwrap();

    let (_id, json_str) = client
        .raw_request("decodeTx", &Params::Array(vec![arg_value]))
        .await
        .expect("raw_request success");
    log::info!("decodeTx request: {}", json_str);
    let req = http::request::Builder::new()
        .body(json_str.as_bytes().to_vec())
        .unwrap();
    let resp = http_client.serve_http_simple(req).await.map_err(|e| {
        Error::new(
            ErrorKind::Other,
            format!("failed to initialize vm: {:?}", e),
        )
    });
    assert!(resp.is_ok());
    let resp = resp.unwrap();
    let body = std::str::from_utf8(&resp.body()).unwrap();
    log::info!("decode response {}", body);

    // TODO shutdown;
}

#[derive(Clone)]
struct MockAppSender;

impl MockAppSender {
    fn new() -> Box<dyn avalanche_types::subnet::rpc::common::appsender::AppSender + Send + Sync> {
        Box::new(MockAppSender {})
    }
}

#[tonic::async_trait]
impl avalanche_types::subnet::rpc::common::appsender::AppSender for MockAppSender {
    async fn send_app_request(
        &self,
        _node_ids: ids::node::Set,
        _request_id: u32,
        _request: Vec<u8>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_app_response(
        &self,
        _node_if: ids::node::Id,
        _request_id: u32,
        _response: Vec<u8>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_app_gossip(&self, _msg: Vec<u8>) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_app_gossip_specific(
        &self,
        _node_ids: ids::node::Set,
        _msg: Vec<u8>,
    ) -> std::io::Result<()> {
        Ok(())
    }

    async fn send_cross_chain_app_request(
        &self,
        _chain_id: ids::Id,
        _request_id: u32,
        _app_request_bytes: Vec<u8>,
    ) -> std::io::Result<()> {
        Ok(())
    }
    async fn send_cross_chain_app_response(
        &self,
        _chain_id: ids::Id,
        _request_id: u32,
        _app_response_bytes: Vec<u8>,
    ) -> std::io::Result<()> {
        Ok(())
    }
}
