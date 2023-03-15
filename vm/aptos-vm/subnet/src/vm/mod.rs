use std::{collections::{HashMap, VecDeque}, io::{self, Error, ErrorKind}, sync::Arc, thread};
use std::sync::atomic::AtomicBool;
use std::time::{SystemTime, UNIX_EPOCH};

use avalanche_types::{
    choices, ids,
    subnet::{self, rpc::snow},
};
use avalanche_types::subnet::rpc::database::BoxedDatabase;
use avalanche_types::subnet::rpc::database::manager::versioned_database::VersionedDatabase;
use avalanche_types::subnet::rpc::snow::engine::common::http_handler;
use chrono::{DateTime, Utc};
use hex;
use jsonrpc_core::IoHandler;
use rand::SeedableRng;
use tokio::sync::{mpsc::Sender, RwLock, RwLockWriteGuard};
use aptos_crypto::hash::CryptoHash;

use aptos_crypto::HashValue;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::{BlockExecutorTrait, StateComputeResult};
use aptos_sdk::transaction_builder::{aptos_stdlib, TransactionFactory};
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_state_view::account_with_state_view::{AccountWithStateView, AsAccountWithStateView};
use aptos_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::aptos_test_root_address;
use aptos_types::account_view::AccountView;
use aptos_types::block_info::BlockInfo;
use aptos_types::block_metadata::BlockMetadata;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures};
use aptos_types::transaction::{Transaction, WriteSetPayload};
use aptos_types::transaction::Transaction::UserTransaction;
use aptos_types::trusted_state::TrustedState;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::waypoint::Waypoint;
use aptos_vm::{AptosVM, VMExecutor};
use aptos_vm_genesis::{GENESIS_KEYPAIR, test_genesis_change_set_and_validators};

use crate::{api, block::Block, state};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Limits how much data a user can propose.
pub const PROPOSE_LIMIT_BYTES: usize = 1024 * 1024;

/// Represents VM-specific states.
/// Defined in a separate struct, for interior mutability in [`Vm`](Vm).
/// To be protected with `Arc` and `RwLock`.
pub struct VmState {
    pub ctx: Option<subnet::rpc::context::Context>,

    /// Represents persistent Vm state.
    pub state: Option<state::State>,
    /// Currently preferred block Id.
    pub preferred: ids::Id,
    /// Channel to send messages to the snowman consensus engine.
    pub to_engine: Option<Sender<snow::engine::common::message::Message>>,
    /// Set "true" to indicate that the Vm has finished bootstrapping
    /// for the chain.
    pub bootstrapped: bool,

    pub db: Option<DbReaderWriter>,
    pub signer: Option<ValidatorSigner>,
    pub executor: Option<BlockExecutor<AptosVM, Transaction>>,

}

impl Default for VmState {
    fn default() -> Self {
        Self {
            ctx: None,
            state: None,
            signer: None,
            preferred: ids::Id::empty(),
            to_engine: None,
            executor: None,
            db: None,
            bootstrapped: false,
        }
    }
}

/// Implements [`snowman.block.ChainVM`](https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#ChainVM) interface.
#[derive(Clone)]
pub struct Vm {
    /// Maintains the Vm-specific states.
    pub state: Arc<RwLock<VmState>>,
    pub app_sender: Option<Box<dyn snow::engine::common::appsender::AppSender + Send + Sync>>,
    pub round: Arc<RwLock<u64>>,

    /// A queue of data that have not been put into a block and proposed yet.
    /// Mempool is not persistent, so just keep in memory via Vm.
    pub mempool: Arc<RwLock<VecDeque<Vec<u8>>>>,

    pub accounts: Arc<RwLock<Vec<Vec<u8>>>>,

}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(VmState::default())),
            app_sender: None,
            mempool: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            accounts: Arc::new(RwLock::new(Vec::new())),
            round: Arc::new(RwLock::new(1)),
        }
    }

    pub async fn is_bootstrapped(&self) -> bool {
        let vm_state = self.state.read().await;
        vm_state.bootstrapped
    }

    /// Signals the consensus engine that a new block is ready to be created.
    pub async fn notify_block_ready(&self) {
        let vm_state = self.state.read().await;
        if let Some(to_engine) = &vm_state.to_engine {
            to_engine
                .send(snow::engine::common::message::Message::PendingTxs)
                .await
                .unwrap_or_else(|e| log::warn!("dropping message to consensus engine: {}", e));

            log::info!("notified block ready!");
        } else {
            log::error!("consensus engine channel failed to initialized");
        }
    }

    /// Proposes arbitrary data to mempool and notifies that a block is ready for builds.
    /// Other VMs may optimize mempool with more complicated batching mechanisms.
    pub async fn propose_block(&mut self, d: Vec<u8>) -> io::Result<()> {
        let size = d.len();
        log::info!("proposed {size} bytes of data for a block");
        let mut mempool = self.mempool.write().await;
        mempool.push_back(d);

        self.notify_block_ready().await;
        Ok(())
    }

    /// Sets the state of the Vm.
    pub async fn set_state(&self, snow_state: snow::State) -> io::Result<()> {
        let mut vm_state = self.state.write().await;
        match snow_state {
            // called by chains manager when it is creating the chain.
            snow::State::Initializing => {
                log::info!("set_state: initializing");
                vm_state.bootstrapped = false;
                Ok(())
            }

            snow::State::StateSyncing => {
                log::info!("set_state: state syncing");
                Err(Error::new(ErrorKind::Other, "state sync is not supported"))
            }

            // called by the bootstrapper to signal bootstrapping has started.
            snow::State::Bootstrapping => {
                log::info!("set_state: bootstrapping");
                vm_state.bootstrapped = false;
                Ok(())
            }

            // called when consensus has started signalling bootstrap phase is complete.
            snow::State::NormalOp => {
                log::info!("set_state: normal op");
                vm_state.bootstrapped = true;
                Ok(())
            }
        }
    }

    /// Sets the container preference of the Vm.
    pub async fn set_preference(&self, id: ids::Id) -> io::Result<()> {
        let mut vm_state = self.state.write().await;
        vm_state.preferred = id;

        Ok(())
    }

    /// Returns the last accepted block Id.
    pub async fn last_accepted(&self) -> io::Result<ids::Id> {
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let blk_id = state.get_last_accepted_block_id().await?;
            return Ok(blk_id);
        }
        Err(Error::new(ErrorKind::NotFound, "state manager not found"))
    }

    pub fn get_core_account() -> LocalAccount {
        LocalAccount::new(
            aptos_test_root_address(),
            AccountKey::from_private_key(GENESIS_KEYPAIR.0.clone()),
            0,
        )
    }

    async fn init_aptos(&self) {
        let mut vm_state = self.state.write().await;
        let (genesis, validators) = test_genesis_change_set_and_validators(Some(1));
        let signer = ValidatorSigner::new(
            validators[0].data.owner_address,
            validators[0].consensus_key.clone(),
        );
        vm_state.signer = Some(signer.clone());
        let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
        let path = TempPath::new();
        path.create_as_dir().unwrap();
        let db = DbReaderWriter::wrap(
            AptosDB::new_for_test(&path));
        let waypoint = generate_waypoint::<AptosVM>(&db.1, &genesis_txn).unwrap();
        maybe_bootstrap::<AptosVM>(&db.1, &genesis_txn, waypoint).unwrap();
        let executor = BlockExecutor::new(db.1.clone());
        vm_state.executor = Some(executor);
        vm_state.db = Some(db.1);
        drop(vm_state);
    }
}

impl subnet::rpc::vm::Vm for Vm {}

#[tonic::async_trait]
impl snow::engine::common::vm::Vm for Vm {
    async fn initialize(
        &mut self,
        ctx: Option<subnet::rpc::context::Context>,
        db_manager: Box<dyn subnet::rpc::database::manager::Manager + Send + Sync>,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: Sender<snow::engine::common::message::Message>,
        _fxs: &[snow::engine::common::vm::Fx],
        app_sender: Box<dyn snow::engine::common::appsender::AppSender + Send + Sync>,
    ) -> io::Result<()> {
        log::info!("initializing Vm");
        let mut vm_state = self.state.write().await;
        vm_state.ctx = ctx;
        let current = db_manager.current().await?;
        let state = state::State {
            db: Arc::new(RwLock::new(current.clone().db)),
            verified_blocks: Arc::new(RwLock::new(HashMap::new())),
        };
        vm_state.state = Some(state.clone());
        vm_state.to_engine = Some(to_engine);
        self.app_sender = Some(app_sender);
        let genesis = "hello world";
        let has_last_accepted = state.has_last_accepted_block().await?;
        if has_last_accepted {
            let last_accepted_blk_id = state.get_last_accepted_block_id().await?;
            vm_state.preferred = last_accepted_blk_id;
            log::info!("initialized Vm with last accepted block {last_accepted_blk_id}");
        } else {
            let mut genesis_block = Block::new(
                ids::Id::empty(),
                0,
                0,
                genesis.as_bytes().to_vec(),
                choices::status::Status::default(),
            ).unwrap();
            genesis_block.set_state(state.clone());
            genesis_block.accept().await?;

            let genesis_blk_id = genesis_block.id();
            vm_state.preferred = genesis_blk_id;
            log::info!("initialized Vm with genesis block {genesis_blk_id}");
        }

        self.mempool = Arc::new(RwLock::new(VecDeque::with_capacity(100)));

        log::info!("successfully initialized Vm");
        Ok(())
    }

    async fn set_state(&self, snow_state: snow::State) -> io::Result<()> {
        self.set_state(snow_state).await
    }

    /// Called when the node is shutting down.
    async fn shutdown(&self) -> io::Result<()> {
        // grpc servers are shutdown via broadcast channel
        // if additional shutdown is required we can extend.
        Ok(())
    }

    async fn version(&self) -> io::Result<String> {
        Ok(String::from(VERSION))
    }

    async fn create_static_handlers(
        &mut self,
    ) -> io::Result<HashMap<String, snow::engine::common::http_handler::HttpHandler>> {
        let svc = api::static_handlers::Service::new(self.clone());
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(api::static_handlers::Rpc::to_delegate(svc));

        let http_handler = snow::engine::common::http_handler::HttpHandler::new_from_u8(0, handler)
            .map_err(|_| Error::from(ErrorKind::InvalidData))?;

        let mut handlers = HashMap::new();
        handlers.insert("/static".to_string(), http_handler);
        Ok(handlers)
    }

    async fn create_handlers(
        &mut self,
    ) -> io::Result<HashMap<String, snow::engine::common::http_handler::HttpHandler>> {
        let svc = api::chain_handlers::Service::new(self.clone());
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(api::chain_handlers::Rpc::to_delegate(svc));

        let http_handler = snow::engine::common::http_handler::HttpHandler::new_from_u8(0, handler)
            .map_err(|_| Error::from(ErrorKind::InvalidData))?;
        let mut handlers = HashMap::new();
        handlers.insert("/rpc".to_string(), http_handler);
        Ok(handlers)
    }
}


fn get_account_balance(account_state_view: &AccountWithStateView) -> u64 {
    account_state_view
        .get_coin_store_resource()
        .unwrap()
        .map(|b| b.coin())
        .unwrap_or(0)
}

#[tonic::async_trait]
impl subnet::rpc::snowman::block::ChainVm for Vm {
    /// Builds a block from mempool data.
    async fn build_block(
        &self,
    ) -> io::Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        let mut mempool = self.mempool.write().await;
        log::info!("build_block called for {} mempool", mempool.len());
        if mempool.is_empty() {
            return Err(Error::new(ErrorKind::Other, "no pending block"));
        }
        let round_1 = self.round.read().await;
        let round = *round_1;
        log::info!("current round {}",round);
        if round == 1 {
            self.init_aptos().await;
        }
        drop(round_1);
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let prnt_blk = state.get_block(&vm_state.preferred).await?;
            let unix_now = Utc::now().timestamp() as u64;
            let first = mempool.pop_front().unwrap();
            let mut block_ = Block::new(
                prnt_blk.id(),
                prnt_blk.height() + 1,
                unix_now,
                first,
                choices::status::Status::Processing,
            )?;
            block_.set_state(state.clone());
            block_.verify().await?;
            log::info!("successfully built block");
            let state = self.state.read().await;
            let executor = state.executor.as_ref().unwrap();
            let signer = state.signer.as_ref().unwrap();
            let db = state.db.as_ref().unwrap();
            let mut accounts = self.accounts.write().await;


            const B: u64 = 1_000_000_000;

            let mut rng = ::rand::rngs::StdRng::from_entropy();
            let mut account = LocalAccount::generate(&mut rng);
            let acc_bytes = account.address().clone().to_vec();

            let mut rng = ::rand::rngs::StdRng::from_entropy();
            let account2 = LocalAccount::generate(&mut rng);
            let acc_bytes2 = account2.address().clone().to_vec();

            accounts.push(acc_bytes.clone());
            accounts.push(acc_bytes2.clone());

            log::info!("------acc_bytes---{}---- accounts {} ", hex::encode(acc_bytes), accounts.len().to_string());


            let latest_ledger_info = db.reader.get_latest_ledger_info().unwrap();
            let next_epoch = latest_ledger_info.ledger_info().next_block_epoch();
            let now = SystemTime::now();
            let since_the_epoch = now.duration_since(UNIX_EPOCH).unwrap();
            let block_id = HashValue::random();
            let block_meta = Transaction::BlockMetadata(BlockMetadata::new(
                block_id,
                next_epoch,
                0,
                signer.author(),
                vec![],
                vec![],
                since_the_epoch.as_secs(),
            ));
            log::info!("------block_id---{}----",block_id );
            let mut core_account = Vm::get_core_account();
            let tx_factory = TransactionFactory::new(ChainId::test());
            let tx_acc_create = core_account.sign_with_transaction_builder(
                tx_factory.create_user_account(account.public_key()));
            let tx_acc_mint = core_account
                .sign_with_transaction_builder(tx_factory.mint(account.address(), 1_00 * B));

            let tx_acc_create_2 = core_account.sign_with_transaction_builder(
                tx_factory.create_user_account(account2.public_key()));
            let tx_acc_transfer_2 = account
                .sign_with_transaction_builder(tx_factory.transfer(account2.address(), 20 * B));

            let block: Vec<_> = vec![
                block_meta,
                UserTransaction(tx_acc_create),
                UserTransaction(tx_acc_mint),
                Transaction::StateCheckpoint(HashValue::random()),
            ];
            let parent_block_id = executor.committed_block_id();
            log::info!(" parent_block_id  {} ",  parent_block_id);
            let output = executor
                .execute_block((block_id, block.clone()), parent_block_id)
                .unwrap();

            let ledger_info = LedgerInfo::new(
                BlockInfo::new(
                    next_epoch,
                    0,
                    block_id,
                    output.root_hash(),
                    output.version(),
                    since_the_epoch.as_secs(),
                    output.epoch_state().clone(),
                ),
                HashValue::zero(),
            );
            let li = generate_ledger_info_with_sig(&[signer.clone()], ledger_info);

            executor.commit_blocks(vec![block_id], li).unwrap();
            let latest_ledger_info = db.reader.get_latest_ledger_info().unwrap();
            let next_epoch = latest_ledger_info.ledger_info().next_block_epoch();
            let ts_2 = since_the_epoch.as_secs() + 1;
            let block_id = HashValue::random();
            let block_meta = Transaction::BlockMetadata(BlockMetadata::new(
                block_id,
                next_epoch,
                0,
                signer.author(),
                vec![],
                vec![],
                ts_2,
            ));
            let block: Vec<_> = vec![
                block_meta,
                UserTransaction(tx_acc_create_2),
                UserTransaction(tx_acc_transfer_2),
                Transaction::StateCheckpoint(HashValue::random()),
            ];
            let parent_block_id = executor.committed_block_id();

            log::info!(" parent_block_id  {} ",  parent_block_id);
            let output = executor
                .execute_block((block_id, block.clone()), parent_block_id).unwrap();
            let ledger_info = LedgerInfo::new(
                BlockInfo::new(
                    next_epoch,
                    0,
                    block_id,
                    output.root_hash(),
                    output.version(),
                    ts_2,
                    output.epoch_state().clone(),
                ),
                HashValue::zero(),
            );
            let li = generate_ledger_info_with_sig(&[signer.clone()], ledger_info);
            executor.commit_blocks(vec![block_id], li).unwrap();


            let state_proof = db.reader.get_latest_ledger_info().unwrap();
            let current_version = state_proof.ledger_info().version();
            log::info!(" current version  {} ",  current_version);

            let block_info = db.reader.get_block_info_by_version(current_version).unwrap();
            log::info!("block hash {}", block_info.2.hash().unwrap());
            let db_state_view = db.reader.state_view_at_version(Some(current_version)).unwrap();
            {
                let acc_address = account.address();
                let account_view = db_state_view.as_account_with_state_view(&acc_address);
                let bal = get_account_balance(&account_view);
                log::info!("{acc_address} balance is  {} ", bal);

                let acc_address_2 = account2.address();
                let account_view = db_state_view.as_account_with_state_view(&acc_address_2);
                let bal = get_account_balance(&account_view);
                log::info!("{acc_address_2} balance is  {} ", bal);
            }

            // for acc in accounts.clone() {
            //     let account = AccountAddress::from_bytes(acc.clone()).unwrap();
            //     let account_view = db_state_view.as_account_with_state_view(&account);
            //     let bal = get_account_balance(&account_view);
            //     log::info!(" this account {} balance is  {} ", hex::encode(acc), bal)
            // }
            let mut round_1 = self.round.write().await;
            *round_1 = round + 2;
            return Ok(Box::new(block_));
        }
        Err(Error::new(
            ErrorKind::Other,
            "not implement",
        ))
    }

    async fn issue_tx(
        &self,
    ) -> io::Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "issue_tx not implemented",
        ))
    }

    async fn set_preference(&self, id: ids::Id) -> io::Result<()> {
        self.set_preference(id).await
    }

    async fn last_accepted(&self) -> io::Result<ids::Id> {
        self.last_accepted().await
    }
}

#[tonic::async_trait]
impl snow::engine::common::engine::NetworkAppHandler for Vm {
    /// Currently, no app-specific messages, so returning Ok.
    async fn app_request(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _deadline: DateTime<Utc>,
        _request: &[u8],
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no app-specific messages, so returning Ok.
    async fn app_request_failed(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no app-specific messages, so returning Ok.
    async fn app_response(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _response: &[u8],
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no app-specific messages, so returning Ok.
    async fn app_gossip(&self, _node_id: &ids::node::Id, _msg: &[u8]) -> io::Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl snow::engine::common::engine::CrossChainAppHandler for Vm {
    /// Currently, no cross chain specific messages, so returning Ok.
    async fn cross_chain_app_request(
        &self,
        _chain_id: &ids::Id,
        _request_id: u32,
        _deadline: DateTime<Utc>,
        _request: &[u8],
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no cross chain specific messages, so returning Ok.
    async fn cross_chain_app_request_failed(
        &self,
        _chain_id: &ids::Id,
        _request_id: u32,
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no cross chain specific messages, so returning Ok.
    async fn cross_chain_app_response(
        &self,
        _chain_id: &ids::Id,
        _request_id: u32,
        _response: &[u8],
    ) -> io::Result<()> {
        Ok(())
    }
}

impl snow::engine::common::engine::AppHandler for Vm {}

#[tonic::async_trait]
impl snow::engine::common::vm::Connector for Vm {
    async fn connected(&self, _id: &ids::node::Id) -> io::Result<()> {
        // no-op
        Ok(())
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> io::Result<()> {
        // no-op
        Ok(())
    }
}

#[tonic::async_trait]
impl subnet::rpc::health::Checkable for Vm {
    async fn health_check(&self) -> io::Result<Vec<u8>> {
        Ok("200".as_bytes().to_vec())
    }
}

#[tonic::async_trait]
impl subnet::rpc::snowman::block::Getter for Vm {
    async fn get_block(
        &self,
        blk_id: ids::Id,
    ) -> io::Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        log::info!("get_block called");
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let block = state.get_block(&blk_id).await?;
            return Ok(Box::new(block));
        }
        Err(Error::new(ErrorKind::NotFound, "state manager not found"))
    }
}

#[tonic::async_trait]
impl subnet::rpc::snowman::block::Parser for Vm {
    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> io::Result<Box<dyn subnet::rpc::consensus::snowman::Block + Send + Sync>> {
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let mut new_block = Block::from_slice(bytes)?;
            new_block.set_status(choices::status::Status::Processing);
            new_block.set_state(state.clone());
            log::info!("parsed block {}", new_block.id());

            return match state.get_block(&new_block.id()).await {
                Ok(prev) => {
                    Ok(Box::new(prev))
                }
                Err(_) => Ok(Box::new(new_block)),
            };
        }

        Err(Error::new(ErrorKind::NotFound, "state manager not found"))
    }
}
