use std::{collections::{HashMap, VecDeque}, io::{self, Error, ErrorKind}, sync::Arc, thread};
use std::collections::HashSet;
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::{SystemTime, UNIX_EPOCH};

use avalanche_types::{
    choices, ids,
    subnet::{self, rpc::snow},
};
use avalanche_types::proto::google::protobuf::field_descriptor_proto::Type::Uint64;
use avalanche_types::subnet::rpc::database::BoxedDatabase;
use avalanche_types::subnet::rpc::database::manager::versioned_database::VersionedDatabase;
use avalanche_types::subnet::rpc::snow::engine::common::http_handler;
use chrono::{DateTime, Utc};
use futures::{channel::mpsc as futures_mpsc, StreamExt};
use hex;
use jsonrpc_core::IoHandler;
use rand::{Rng, SeedableRng};
use serde_json::json;
use tokio::sync::{broadcast, mpsc, mpsc::Sender, RwLock, RwLockWriteGuard};
use tonic::IntoRequest;

use aptos_api::{Context, get_api_service, get_raw_api_service, RawApi};
use aptos_api::accept_type::AcceptType;
use aptos_api::response::{AptosResponseContent, BasicResponse, BasicResultWith404};
use aptos_api::response::AptosResponseContent::Json;
use aptos_api::transactions::{SubmitTransactionPost, SubmitTransactionResponse};
use aptos_api::transactions::SubmitTransactionPost::Bcs;
use aptos_api_types::{Address, MoveStructTag, TransactionOnChainData, ViewRequest};
use aptos_config::config::NodeConfig;
use aptos_crypto::{HashValue, ValidCryptoMaterialStringExt};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::x25519::PublicKey;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::{BlockExecutorTrait, StateComputeResult};
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_mempool::core_mempool::CoreMempool;
use aptos_sdk::rest_client::aptos_api_types::{AsConverter, MAX_RECURSIVE_TYPES_ALLOWED};
use aptos_sdk::transaction_builder::{aptos_stdlib, TransactionFactory};
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_state_view::account_with_state_view::{AccountWithStateView, AsAccountWithStateView};
use aptos_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use aptos_storage_interface::state_view::{DbStateViewAtVersion, LatestDbStateCheckpointView};
use aptos_temppath::TempPath;
use aptos_types::access_path::Path;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::{AccountResource, aptos_test_root_address};
use aptos_types::account_view::AccountView;
use aptos_types::block_info::BlockInfo;
use aptos_types::block_metadata::BlockMetadata;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures};
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::{SignedTransaction, Transaction, TransactionOutput, TransactionWithProof, WriteSetPayload};
use aptos_types::transaction::ExecutionStatus::Success;
use aptos_types::transaction::Transaction::UserTransaction;
use aptos_types::trusted_state::TrustedState;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::waypoint::Waypoint;
use aptos_vm::{AptosVM, VMExecutor};
use aptos_vm::data_cache::IntoMoveResolver;
use aptos_vm_genesis::{GENESIS_KEYPAIR, test_genesis_change_set_and_validators};

use crate::{api, block::Block, state};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    pub mempool: Arc<RwLock<VecDeque<SignedTransaction>>>,

    pub accounts: Arc<RwLock<Vec<Vec<u8>>>>,

    pub api_service: Option<RawApi>,

    pub api_context: Option<Context>,

    pub core_mempool: Option<Arc<RwLock<CoreMempool>>>,
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
            api_service: None,
            api_context: None,
            core_mempool: None,
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

    pub async fn get_accounts_transactions(&self, account: &str) -> String {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.3.get_account_resources_raw(AcceptType::Json,
                                                  Address::from_str(account).unwrap()).await.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn get_account_resources(&self, account: &str) -> String {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.3.get_account_resources_raw(AcceptType::Json,
                                                  Address::from_str(account).unwrap()).await.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn get_account(&self, account: &str) -> String {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.3.get_account_raw(AcceptType::Json,
                                        Address::from_str(account).unwrap(), None).await.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn get_account_resources_state(&self, account: &str, resource: &str) -> String {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.4.get_account_resource_raw(AcceptType::Json,
                                                 Address::from_str(account).unwrap(),
                                                 MoveStructTag::from_str(resource).unwrap(), None).await.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn get_account_modules(&self, account: &str) -> String {
        let api = self.api_service.as_ref().unwrap();
        let address = Address::from_str(account).unwrap();
        log::info!("-----get_account_modules---{}----",address);
        let ret = api.3.get_account_modules_raw(AcceptType::Json,
                                                address,
                                                None,
                                                None,
                                                None).await.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        log::info!("-----get_account_modules-length--{}----",ret.clone().len());
        ret
    }

    pub async fn get_ledger_info(&self) -> String {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.2.get_ledger_info_raw(AcceptType::Json).await.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn view_function(&self, req: &str) -> String {
        let api = self.api_service.as_ref().unwrap();
        let req = serde_json::from_str::<ViewRequest>(req).unwrap();
        let ret = api.1.view_function_raw(AcceptType::Json, req, None).await;
        let ret = ret.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }
    pub async fn get_transaction_by_hash(&self, h: &str) -> String {
        let h1 = HashValue::from_hex(h).unwrap();
        let hash = aptos_api_types::hash::HashValue::from(h1);
        let api = self.api_service.as_ref().unwrap();
        let ret = api.0.get_transaction_by_hash_raw(AcceptType::Json,
                                                    hash).await;
        let ret = ret.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn submit_transaction(&self, data: Vec<u8>) -> Vec<u8> {
        let mut mempool = self.mempool.write().await;
        log::info!("submit_transaction length {}",{data.len()});
        let service = self.api_service.as_ref().unwrap();
        let context = self.api_context.as_ref().unwrap();
        let info = context.get_latest_ledger_info_wrapped().unwrap();
        let signed_transaction = service.0.get_signed_transaction(
            &info, Bcs(aptos_api::bcs_payload::Bcs(data))).unwrap();
        let hash = signed_transaction.clone().committed_hash().to_vec();
        mempool.push_back(signed_transaction);
        self.notify_block_ready().await;
        hash
    }

    pub async fn submit_transaction2(&self, data: Vec<u8>) -> String {
        log::info!("submit_transaction length {}",{data.len()});
        let service = self.api_service.as_ref().unwrap();
        let payload = Bcs(aptos_api::bcs_payload::Bcs(data.clone()));
        let ret = service.0.submit_transaction_raw(AcceptType::Json,
                                                   payload).await;
        let ret = ret.unwrap();
        let ret = match ret {
            SubmitTransactionResponse::Accepted(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
            _ => {
                format!("{}", String::from("not found"))
            }
        };
        let mut mempool = self.mempool.write().await;
        let signed_transaction: SignedTransaction =
            bcs::from_bytes_with_limit(&data,
                                       MAX_RECURSIVE_TYPES_ALLOWED as usize).unwrap();
        mempool.push_back(signed_transaction);
        self.notify_block_ready().await;
        ret
    }

    pub async fn simulate_transaction(&self, data: Vec<u8>) -> String {
        let service = self.api_service.as_ref().unwrap();
        let ret = service.0.simulate_transaction_raw(
            AcceptType::Json,
            Some(true),
            Some(false),
            Some(true), Bcs(aptos_api::bcs_payload::Bcs(data))).await;
        let ret = ret.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn estimate_gas_price(&self) -> String {
        let service = self.api_service.as_ref().unwrap();
        let ret = service.0.estimate_gas_price_raw(
            AcceptType::Json).await;
        let ret = ret.unwrap();
        let ret = match ret {
            BasicResponse::Ok(c, ..) => {
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", String::from("hello"))
                    }
                }
            }
        };
        ret
    }

    pub async fn facet_apt(&self, acc: Vec<u8>) -> Vec<u8> {
        let to = AccountAddress::from_bytes(acc).unwrap();
        let vm_state = self.state.read().await;
        let db = vm_state.db.as_ref().unwrap();
        let mut core_account = self.get_core_account(db).await;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_mint = core_account
            .sign_with_transaction_builder(
                tx_factory.mint(to, 1000 * 100_000_000)
            );
        let hash = tx_acc_mint.clone().committed_hash().to_vec();
        let mut mempool = self.mempool.write().await;
        mempool.push_back(tx_acc_mint);
        self.notify_block_ready().await;
        hash
    }

    pub async fn create_account(&self, key: &str) -> Vec<u8> {
        let to = Ed25519PublicKey::from_encoded_string(key).unwrap();
        let vm_state = self.state.read().await;
        let db = vm_state.db.as_ref().unwrap();
        let mut core_account = self.get_core_account(db).await;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_mint = core_account
            .sign_with_transaction_builder(
                tx_factory.create_user_account(&to)
            );
        let hash = tx_acc_mint.clone().committed_hash().to_vec();
        let mut mempool = self.mempool.write().await;
        mempool.push_back(tx_acc_mint);
        self.notify_block_ready().await;
        hash
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

    pub async fn get_core_account(&self, db: &DbReaderWriter) -> LocalAccount {
        let core_account = LocalAccount::new(
            aptos_test_root_address(),
            AccountKey::from_private_key(GENESIS_KEYPAIR.0.clone()),
            0,
        );
        let addr = core_account.address();
        let av = self.get_account_resource_me(db, addr.as_ref());
        let sn = av.unwrap().sequence_number();
        LocalAccount::new(
            aptos_test_root_address(),
            AccountKey::from_private_key(GENESIS_KEYPAIR.0.clone()),
            sn,
        )
    }


    pub fn get_account_balance(&self, db: &DbReaderWriter, acc: &[u8]) -> u64 {
        let state_proof = db.reader.get_latest_ledger_info().unwrap();
        let current_version = state_proof.ledger_info().version();
        let db_state_view = db.reader.state_view_at_version(Some(current_version)).unwrap();
        let account = AccountAddress::from_bytes(acc).unwrap();
        let view = db_state_view.as_account_with_state_view(&account);
        view
            .get_coin_store_resource()
            .unwrap()
            .map(|b| b.coin())
            .unwrap_or(0)
    }

    pub fn get_account_resource_me(&self, db: &DbReaderWriter, acc: &[u8]) -> Option<AccountResource> {
        let state_proof = db.reader.get_latest_ledger_info().unwrap();
        let current_version = state_proof.ledger_info().version();
        let db_state_view = db.reader.state_view_at_version(Some(current_version)).unwrap();
        let account = AccountAddress::from_bytes(acc).unwrap();
        let view = db_state_view.as_account_with_state_view(&account);
        view.get_account_resource().unwrap()
    }

    async fn init_aptos(&mut self) {
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
        vm_state.db = Some(db.1.clone());
        let (mempool_client_sender,
            mut mempool_client_receiver) = futures_mpsc::channel::<MempoolClientRequest>(10);
        let sender = MempoolClientSender::from(mempool_client_sender);
        let node_config = NodeConfig::default();
        let context = Context::new(ChainId::test(),
                                   db.1.reader.clone(),
                                   sender, node_config.clone());
        self.api_context = Some(context.clone());
        let service = get_raw_api_service(Arc::new(context));
        self.api_service = Some(service);
        self.core_mempool = Some(Arc::new(RwLock::new(CoreMempool::new(&node_config))));
        drop(vm_state);
        tokio::task::spawn(async move {
            while let Some(request) = mempool_client_receiver.next().await {
                log::info!("-----mempool_client_receiver-SubmitTransaction--");
                match request {
                    MempoolClientRequest::SubmitTransaction(t, callback) => {
                        let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
                        let status: SubmissionStatus = (ms, None);
                        callback.send(
                            Ok(status)
                        ).unwrap();
                        log::info!("--SubmitTransaction--");
                    }
                    MempoolClientRequest::GetTransactionByHash(_, _) => {}
                }
            }
        });
    }
}

impl subnet::rpc::vm::Vm for Vm {}

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
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let prnt_blk = state.get_block(&vm_state.preferred).await?;
            let unix_now = Utc::now().timestamp() as u64;
            let first = mempool.pop_front().unwrap();
            let mut block_ = Block::new(
                prnt_blk.id(),
                prnt_blk.height() + 1,
                unix_now,
                vec![],
                choices::status::Status::Processing,
            )?;
            block_.set_state(state.clone());
            block_.verify().await?;
            let state = self.state.read().await;
            let executor = state.executor.as_ref().unwrap();
            let signer = state.signer.as_ref().unwrap();
            let db = state.db.as_ref().unwrap();
            let latest_ledger_info = db.reader.get_latest_ledger_info().unwrap();
            let next_epoch = latest_ledger_info.ledger_info().next_block_epoch();
            log::info!("------next_epoch---{}----",next_epoch );
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
            let mut txs = vec![
                UserTransaction(first)
            ];
            // let a = db.reader.latest_state_checkpoint_view().map(|state_view| state_view.into_move_resolver()).unwrap();
            // let v = a.as_converter(db.reader.clone());
            // for t in txs.clone().into_iter() {
            //     let sign_t = t.as_signed_user_txn().unwrap();
            //     let pt = v.try_into_pending_transaction_poem(sign_t.clone()).unwrap();
            //     println!("--------pending transaction---------{}-", pt.hash);
            // }
            let mut block: Vec<_> = vec![];
            block.push(block_meta);
            block.append(&mut txs);
            block.push(Transaction::StateCheckpoint(HashValue::random()));
            let parent_block_id = executor.committed_block_id();
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
            log::info!("successfully built block");
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
    async fn app_gossip(&self, _node_id: &ids::node::Id, msg: &[u8]) -> io::Result<()> {
        let s = std::str::from_utf8(msg).unwrap().to_string();
        log::info!("app_gossip----->{}", s);
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
        drop(vm_state);
        self.init_aptos().await;
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