pub mod inner;

use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    sync::Arc,
};

use avalanche_types::{
    choices::status::{self, Status},
    ids,
    subnet::{
        self,
        rpc::consensus::snowman::{Block, Initializer},
    },
};
use chrono::{DateTime, Utc};
use tokio::sync::{mpsc, RwLock};

use crate::{
    api, block,
    chain::{self, storage, vm::Vm},
    genesis::Genesis,
    network,
};

pub const PUBLIC_API_ENDPOINT: &str = "/public";
const VERSION: &str = env!("CARGO_PKG_VERSION");

// TODO: make configurable
const MEMPOOL_SIZE: u64 = 1024;

pub struct ChainVm {
    pub inner: Arc<RwLock<inner::Inner>>,
    /// ID of this node
    node_id: ids::node::Id,
}

impl ChainVm {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner::Inner::new())),
            node_id: ids::node::Id::default(),
        }
    }
}

impl Default for ChainVm {
    fn default() -> Self {
        Self::new()
    }
}

impl avalanche_types::subnet::rpc::vm::Vm for ChainVm {}

#[tonic::async_trait]
impl crate::chain::vm::Vm for ChainVm {
    async fn is_bootstrapped(&self) -> bool {
        log::debug!("vm::is_bootstrapped called");

        let vm = self.inner.read().await;
        vm.bootstrapped
    }

    async fn submit(&self, mut txs: Vec<chain::tx::tx::Transaction>) -> Result<()> {
        log::debug!("vm::submit called");

        let mut vm = self.inner.write().await;

        log::debug!("vm::submit store called");
        storage::submit(&vm.state.clone(), &mut txs).await?;

        let mempool = &mut vm.mempool;
        log::debug!("vm::submit add to mempool");
        for tx in txs.iter_mut() {
            let _ = mempool.add(tx)?;
        }
        log::debug!("vm::submit complete");

        Ok(())
    }

    /// Sends a signal to the consensus engine that a new block
    /// is ready to be created.
    async fn notify_block_ready(&self) {
        log::debug!("vm::notify_block_ready called");

        let vm = self.inner.read().await;

        if let Some(engine) = &vm.to_engine {
            engine
                .send(subnet::rpc::common::message::Message::PendingTxs)
                .await
                .unwrap_or_else(|e| log::warn!("dropping message to consensus engine: {}", e));
        } else {
            log::error!("consensus engine channel failed to initialized");
        }
    }
}

#[tonic::async_trait]
impl subnet::rpc::common::apphandler::AppHandler for ChainVm {
    async fn app_request(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _deadline: DateTime<Utc>,
        _request: &[u8],
    ) -> Result<()> {
        log::debug!("vm::app_request called");

        // currently no app-specific messages
        Ok(())
    }

    async fn app_request_failed(&self, _node_id: &ids::node::Id, _request_id: u32) -> Result<()> {
        log::debug!("vm::app_request_failed called");

        // currently no app-specific messages
        Ok(())
    }

    async fn app_response(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _response: &[u8],
    ) -> Result<()> {
        log::debug!("vm::app_response called");

        // currently no app-specific messages
        Ok(())
    }

    async fn app_gossip(&self, node_id: &ids::node::Id, msg: &[u8]) -> Result<()> {
        log::debug!("vm::app_gossip called");

        log::debug!(
            "AppGossip message handler sender: {}, receiver: {}, bytes: {}",
            &node_id,
            &self.node_id,
            &msg.len()
        );

        let txs: Vec<chain::tx::tx::Transaction> = match serde_json::from_slice(msg) {
            Ok(body) => body,
            Err(e) => {
                log::error!("failed to deserialize message peer_id: {}: {}", &node_id, e);
                return Ok(());
            }
        };

        match self.submit(txs).await {
            Ok(_) => {
                log::debug!("vm::app_gossip success");
                return Ok(());
            }
            Err(e) => {
                log::error!("failed to submit tx peer_id: {}: {}", &node_id, e);
                return Ok(());
            }
        }

        // only trace error to prevent VMs being shutdown
        // from "AppGossip" returning an error
    }
}

#[tonic::async_trait]
impl subnet::rpc::common::vm::Connector for ChainVm {
    async fn connected(&self, _id: &ids::node::Id) -> Result<()> {
        log::debug!("vm::connected called");

        // no-op
        Ok(())
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> Result<()> {
        log::debug!("vm::disconnected called");

        // no-op
        Ok(())
    }
}

#[tonic::async_trait]
impl subnet::rpc::health::Checkable for ChainVm {
    async fn health_check(&self) -> Result<Vec<u8>> {
        Ok("200".as_bytes().to_vec())
    }
}

#[tonic::async_trait]
impl subnet::rpc::common::vm::Vm for ChainVm {
    /// Initialize this Vm.
    async fn initialize(
        &mut self,
        ctx: Option<subnet::rpc::context::Context>,
        db_manager: Box<dyn subnet::rpc::database::manager::Manager + Send + Sync>,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: mpsc::Sender<subnet::rpc::common::message::Message>,
        _fxs: &[subnet::rpc::common::vm::Fx],
        app_sender: Box<dyn subnet::rpc::common::appsender::AppSender + Send + Sync>,
    ) -> Result<()> {
        log::debug!("vm::initialize called");

        let mut vm = self.inner.write().await;
        let current = db_manager.current().await?;
        let db = current.db.clone();
        let genesis = Genesis::from_json(genesis_bytes)?;

        vm.ctx = ctx;
        vm.to_engine = Some(to_engine);
        vm.app_sender = Some(app_sender);
        vm.state = block::state::State::new(db);
        vm.genesis = genesis;
        self.node_id = vm.ctx.as_ref().expect("inner.ctx").node_id;

        // Attempt to load last accepted
        let has = vm
            .state
            .has_last_accepted()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        // Check if last accepted block exists
        if has {
            let block_id = vm
                .state
                .get_last_accepted()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            let mut block = vm
                .state
                .get_block(block_id)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            vm.preferred = block_id;
            vm.state.set_last_accepted(&mut block).await?;
            log::debug!("initialized vm from last accepted block id: {:?}", block_id)
        } else {
            let mut genesis_block =
                crate::block::Block::new(ids::Id::empty(), 0, genesis_bytes, 0, vm.state.clone());

            let bytes = genesis_block
                .to_bytes()
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            genesis_block
                .init(&bytes, status::Status::Accepted)
                .await
                .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

            let genesis_block_id = genesis_block.id;
            vm.state
                .set_last_accepted(&mut genesis_block)
                .await
                .map_err(|e| {
                    Error::new(ErrorKind::Other, format!("failed to accept block: {:?}", e))
                })?;

            vm.state.set_last_accepted(&mut genesis_block).await?;
            vm.preferred = genesis_block_id;
            log::debug!("initialized from genesis block: {}", genesis_block_id);
        }

        // start the gossip loops
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            network::Push::new(inner).gossip().await;
        });

        // start timed block builder
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            block::builder::Builder::new(inner).build().await;
        });

        Ok(())
    }

    /// Called when the node is shutting down.
    async fn shutdown(&self) -> Result<()> {
        log::debug!("vm::shutdown called");
        let vm = self.inner.read().await;
        let db = vm.state.get_db().await;
        db.close().await?;

        Ok(())
    }

    /// Communicates to Vm the next state phase.
    async fn set_state(&self, snow_state: subnet::rpc::snow::State) -> Result<()> {
        log::debug!("vm::set_state called");

        let mut vm = self.inner.write().await;

        match snow_state {
            // Initializing is set by chain manager when it is creating the chain.
            subnet::rpc::snow::State::Initializing => {
                log::debug!("set_state: initializing");
                vm.bootstrapped = false;
                Ok(())
            }
            subnet::rpc::snow::State::StateSyncing => {
                log::debug!("set_state: state syncing");
                Err(Error::new(ErrorKind::Other, "state sync is not supported"))
            }
            // Bootstrapping is called by the bootstrapper to signal bootstrapping has started.
            subnet::rpc::snow::State::Bootstrapping => {
                log::debug!("set_state: bootstrapping");
                vm.bootstrapped = false;
                Ok(())
            }
            // NormalOp os called when consensus has started signalling bootstrap phase is complete.
            subnet::rpc::snow::State::NormalOp => {
                log::debug!("set_state: normal op");
                vm.bootstrapped = true;
                Ok(())
            }
        }
    }

    /// Returns the version of the VM this node is running.
    async fn version(&self) -> Result<String> {
        log::debug!("vm::version called");

        Ok(String::from(VERSION))
    }

    /// Creates the HTTP handlers for custom Vm network calls
    /// for "ext/vm/[vmId]"
    async fn create_static_handlers(
        &mut self,
    ) -> std::io::Result<
        std::collections::HashMap<String, subnet::rpc::common::http_handler::HttpHandler>,
    > {
        log::debug!("vm::create_static_handlers called");

        Ok(HashMap::new())
    }

    /// Creates the HTTP handlers for custom chain network calls
    /// for "ext/vm/[chainId]"
    async fn create_handlers(
        &mut self,
    ) -> std::io::Result<
        std::collections::HashMap<
            String,
            avalanche_types::subnet::rpc::common::http_handler::HttpHandler,
        >,
    > {
        log::debug!("vm::create_handlers called");

        // Initialize the jsonrpc public service and handler
        let service = api::service::Service::new(self.inner.clone());
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(api::Service::to_delegate(service));

        let http_handler = subnet::rpc::common::http_handler::HttpHandler::new_from_u8(0, handler)
            .map_err(|_| Error::from(ErrorKind::InvalidData))?;

        let mut handlers = HashMap::new();
        handlers.insert(String::from(PUBLIC_API_ENDPOINT), http_handler);

        Ok(handlers)
    }
}

#[tonic::async_trait]
impl subnet::rpc::snowman::block::Getter for ChainVm {
    /// Attempt to load a block.
    async fn get_block(
        &self,
        id: ids::Id,
    ) -> Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::get_block called: {:?}", id);

        let vm = self.inner.read().await;

        // has block been accepted by the vm and cached.
        if let Some(cached) = vm.state.get_accepted_block(id).await {
            log::debug!("vm::get_block found cached accepted block: {:?}", id);
            return Ok(Box::new(cached.to_owned()));
        }

        // has block been verified, but not yet accepted
        if let Some(block) = vm.state.get_verified_block(id).await {
            log::debug!("vm::get_block found accepted block: {:?}", id);
            return Ok(Box::new(block));
        }

        // check on disk state
        let block = vm
            .state
            .get_block(id)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to get block: {}", e)))?;

        // if block on disk it must have been accepted
        let block = vm
            .state
            .parse_block(Some(block.to_owned()), vec![], Status::Accepted)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to parse block: {}", e)))?;

        Ok(Box::new(block))
    }
}

#[tonic::async_trait]
impl subnet::rpc::snowman::block::Parser for ChainVm {
    /// Attempt to create a block from a stream of bytes.
    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::parse_block called: {:?}", bytes);

        let vm = self.inner.read().await;
        let new_block = vm
            .state
            .parse_block(None, bytes.to_vec(), Status::Processing)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to parse block: {}", e)))?;

        log::debug!("parsed block id: {:?}", new_block.id.to_vec());

        match vm.state.get_block(new_block.id).await {
            Ok(old_block) => {
                log::debug!("returning previously parsed block id: {}", old_block.id);
                return Ok(Box::new(old_block));
            }
            Err(_) => return Ok(Box::new(new_block)),
        };
    }
}

#[tonic::async_trait]
impl subnet::rpc::snowman::block::ChainVm for ChainVm {
    /// Attempt to create a new block.
    async fn build_block(
        &self,
    ) -> Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::build_block called!");

        let vm = self.inner.read().await;

        let preferred = vm.preferred;
        let parent = vm
            .state
            .get_block(preferred)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to get block: {}", e)))?;

        log::debug!("vm::build_block parent found!");
        let next_time = Utc::now().timestamp() as u64;

        // new block
        let mut block = crate::block::Block::new(
            parent.id,
            parent.height + 1,
            &[],
            next_time,
            vm.state.clone(),
        );

        log::debug!("vm::build_block mempool len: {}", vm.mempool.len());

        let mut mtxs = vm.mempool.get_txs();
        let mut txs = Vec::new();
        while let Some(entry) = mtxs.pop_back() {
            if let Some(tx) = entry.tx {
                txs.push(tx)
            }
        }
        log::debug!("vm::build_block mempool len: {}", vm.mempool.len());

        block.txs = txs;

        // compute block hash and marshaled representation
        let bytes = block.to_bytes().await;
        block
            .init(&bytes.unwrap(), status::Status::Processing)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to init block: {}", e)))?;

        log::debug!("vm::build_block parent init!");
        // verify block to ensure it is formed correctly
        block
            .verify()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to verify block: {}", e)))?;

        log::debug!("vm::build_block: success: {:?}", block.id.to_vec());

        Ok(Box::new(block))
    }

    /// Notify the Vm of the currently preferred block.
    async fn set_preference(&self, id: ids::Id) -> Result<()> {
        log::debug!("vm::set_preference called");

        let mut vm = self.inner.write().await;
        vm.preferred = id;

        Ok(())
    }

    // Returns the Id of the last accepted block.
    async fn last_accepted(&self) -> Result<ids::Id> {
        log::debug!("vm::last_accepted called");

        let vm = self.inner.read().await;
        let last = vm.state.get_last_accepted().await?;

        Ok(last)
    }

    /// Attempts to issue a transaction into consensus.
    async fn issue_tx(
        &self,
    ) -> Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        log::debug!("vm::issue_tx called");

        Err(Error::new(
            ErrorKind::Unsupported,
            "issue tx not implemented",
        ))
    }
}
