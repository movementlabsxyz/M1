use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{self, ids, proto, subnet};
use prost::bytes::Bytes;
use tokio::{net::TcpListener, sync::mpsc};
use tonic::transport::Channel;

/// Test Vm client which interacts with rpcchainvm server service.
pub struct Client {
    inner: proto::vm::vm_client::VmClient<Channel>,
    pub stop_ch: tokio::sync::broadcast::Sender<()>,
}

impl Client {
    pub fn new(client_conn: Channel) -> Box<dyn subnet::rpc::common::vm::Vm + Send + Sync> {
        // Initialize broadcast stop channel used to terminate gRPC servers during shutdown.
        let (stop_ch, _): (
            tokio::sync::broadcast::Sender<()>,
            tokio::sync::broadcast::Receiver<()>,
        ) = tokio::sync::broadcast::channel(1);

        Box::new(Self {
            inner: avalanche_types::proto::vm::vm_client::VmClient::new(client_conn),
            stop_ch,
        })
    }
}

#[tonic::async_trait]
impl subnet::rpc::common::vm::Vm for Client {
    async fn initialize(
        &mut self,
        _ctx: Option<subnet::rpc::context::Context>,
        _db_manager: Box<dyn subnet::rpc::database::manager::Manager + Send + Sync>,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        _to_engine: mpsc::Sender<subnet::rpc::common::message::Message>,
        _fxs: &[subnet::rpc::common::vm::Fx],
        _app_sender: Box<dyn subnet::rpc::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        // memdb wrapped in rpcdb
        let db = subnet::rpc::database::rpcdb::server::Server::new(
            subnet::rpc::database::memdb::Database::new(),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        log::info!("starting rpcdb server");
        tokio::spawn(async move {
            crate::common::serve_test_database(db, listener)
                .await
                .unwrap();
        });

        let versiondb_servers = proto::vm::VersionedDbServer {
            server_addr: addr.clone().to_string(),
            version: "0.0.7".to_owned(),
        };

        let mut db_servers = Vec::with_capacity(1);
        db_servers.push(versiondb_servers);

        let request = proto::vm::InitializeRequest {
            network_id: 0,
            subnet_id: Bytes::from(ids::Id::empty().to_vec()),
            chain_id: Bytes::from(ids::Id::empty().to_vec()),
            node_id: Bytes::from(ids::node::Id::empty().to_vec()),
            x_chain_id: Bytes::from(ids::Id::empty().to_vec()),
            avax_asset_id: Bytes::from(ids::Id::empty().to_vec()),
            genesis_bytes: Bytes::from(genesis_bytes.to_vec()),
            upgrade_bytes: Bytes::from(""),
            config_bytes: Bytes::from(""),
            db_servers,
            server_addr: addr.to_string(), //dummy
        };

        // in this context we don't care about the response unless its an error
        let resp = self.inner.initialize(request).await.map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("initialize request failed: {:?}", e),
            )
        });
        assert!(resp.is_ok());
        Ok(())
    }

    async fn set_state(&self, _state: subnet::rpc::snow::State) -> Result<()> {
        // TODO:
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        // TODO:
        Ok(())
    }

    async fn version(&self) -> Result<String> {
        Ok(String::new())
    }

    async fn create_static_handlers(
        &mut self,
    ) -> Result<HashMap<String, subnet::rpc::common::http_handler::HttpHandler>> {
        let resp = self
            .inner
            .create_static_handlers(proto::google::protobuf::Empty {})
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("create static handler request failed: {:?}", e),
                )
            })?;

        let resp = resp.into_inner();

        let mut http_handler: HashMap<String, subnet::rpc::common::http_handler::HttpHandler> =
            HashMap::new();

        for h in resp.handlers.iter() {
            let lock_option =
                subnet::rpc::common::http_handler::LockOptions::try_from(h.lock_options)
                    .map_err(|_| Error::new(ErrorKind::Other, "invalid lock option"))?;
            http_handler.insert(
                h.prefix.clone(),
                subnet::rpc::common::http_handler::HttpHandler {
                    lock_option,
                    handler: None,
                    server_addr: Some(h.server_addr.clone()),
                },
            );
        }

        Ok(http_handler)
    }

    async fn create_handlers(
        &mut self,
    ) -> Result<HashMap<String, subnet::rpc::common::http_handler::HttpHandler>> {
        let resp = self
            .inner
            .create_handlers(proto::google::protobuf::Empty {})
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("create handler request failed: {:?}", e),
                )
            })?;

        let resp = resp.into_inner();

        let mut http_handler: HashMap<String, subnet::rpc::common::http_handler::HttpHandler> =
            HashMap::new();

        for h in resp.handlers.iter() {
            let lock_option =
                subnet::rpc::common::http_handler::LockOptions::try_from(h.lock_options)
                    .map_err(|_| Error::new(ErrorKind::Other, "invalid lock option"))?;
            http_handler.insert(
                h.prefix.clone(),
                subnet::rpc::common::http_handler::HttpHandler {
                    lock_option,
                    handler: None,
                    server_addr: Some(h.server_addr.clone()),
                },
            );
        }

        Ok(http_handler)
    }
}

#[tonic::async_trait]
impl subnet::rpc::health::Checkable for Client {
    async fn health_check(&self) -> Result<Vec<u8>> {
        // TODO:
        Ok(Vec::new())
    }
}

#[tonic::async_trait]
impl subnet::rpc::common::vm::Connector for Client {
    async fn connected(&self, _id: &ids::node::Id) -> Result<()> {
        Ok(())
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl subnet::rpc::common::apphandler::AppHandler for Client {
    async fn app_request(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _deadline: chrono::DateTime<chrono::Utc>,
        _request: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    async fn app_request_failed(&self, _node_id: &ids::node::Id, _request_id: u32) -> Result<()> {
        Ok(())
    }

    async fn app_response(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _response: &[u8],
    ) -> Result<()> {
        Ok(())
    }

    async fn app_gossip(&self, _node_id: &ids::node::Id, _msg: &[u8]) -> Result<()> {
        Ok(())
    }
}
