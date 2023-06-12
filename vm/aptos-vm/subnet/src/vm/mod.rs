use std::{collections::HashMap, fs, io::{self, Error, ErrorKind}, sync::Arc};
use std::str::FromStr;
use std::time::{Duration, Instant};
use avalanche_types::{
    choices, ids,
    subnet::{self, rpc::snow},
};
use avalanche_types::subnet::rpc::database::manager::{DatabaseManager, Manager};
use avalanche_types::subnet::rpc::health::Checkable;
use avalanche_types::subnet::rpc::snow::engine::common::appsender::AppSender;
use avalanche_types::subnet::rpc::snow::engine::common::appsender::client::AppSenderClient;
use avalanche_types::subnet::rpc::snow::engine::common::engine::{AppHandler, CrossChainAppHandler, NetworkAppHandler};
use avalanche_types::subnet::rpc::snow::engine::common::http_handler::{HttpHandler, LockOptions};
use avalanche_types::subnet::rpc::snow::engine::common::message::Message::PendingTxs;
use avalanche_types::subnet::rpc::snow::engine::common::vm::{CommonVm, Connector};
use avalanche_types::subnet::rpc::snow::validators::client::ValidatorStateClient;
use avalanche_types::subnet::rpc::snowman::block::{BatchedChainVm, ChainVm, Getter, Parser};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{channel::mpsc as futures_mpsc, StreamExt};
use hex;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use aptos_api::{Context, get_raw_api_service, RawApi};
use aptos_api::accept_type::AcceptType;
use aptos_api::response::{AptosResponseContent, BasicResponse};
use aptos_api::transactions::{SubmitTransactionPost, SubmitTransactionResponse, SubmitTransactionsBatchPost, SubmitTransactionsBatchResponse};
use aptos_api_types::{Address, EncodeSubmissionRequest, IdentifierWrapper, MoveStructTag, RawTableItemRequest, StateKeyWrapper, TableItemRequest, U64, ViewRequest};
use aptos_config::config::NodeConfig;
use aptos_crypto::{HashValue, ValidCryptoMaterialStringExt};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::BlockExecutorTrait;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_mempool::core_mempool::{CoreMempool, TimelineState};
use aptos_sdk::rest_client::aptos_api_types::MAX_RECURSIVE_TYPES_ALLOWED;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_storage_interface::DbReaderWriter;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::aptos_test_root_address;
use aptos_types::account_view::AccountView;
use aptos_types::block_info::BlockInfo;
use aptos_types::block_metadata::BlockMetadata;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures};
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::{SignedTransaction, Transaction, WriteSetPayload};
use aptos_types::transaction::Transaction::UserTransaction;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use aptos_vm_genesis::{GENESIS_KEYPAIR, test_genesis_change_set_and_validators};

use crate::{block::Block, state};
use crate::api::chain_handlers::{AccountStateArgs, ChainHandler, ChainService, RpcEventHandleReq, RpcEventNumReq, RpcReq, RpcRes, RpcTableReq};
use crate::api::static_handlers::{StaticHandler, StaticService};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MOVE_DB_DIR: &str = "aptos-chain-data";

#[derive(Serialize, Deserialize, Clone)]
pub struct AptosData(
    pub Vec<u8>, // block info
    pub HashValue, // block id
    pub HashValue,
    pub u64,
    pub u64,
    pub Vec<u8>,// leger info
);

#[derive(Serialize, Deserialize, Clone)]
pub struct AptosHeader {
    chain_id: u8,
    ledger_version: u64,
    ledger_oldest_version: u64,
    ledger_timestamp_usec: u64,
    epoch: u64,
    block_height: u64,
    oldest_block_height: u64,
    cursor: Option<String>,
}


/// Represents VM-specific states.
/// Defined in a separate struct, for interior mutability in [`Vm`](Vm).
/// To be protected with `Arc` and `RwLock`.
pub struct VmState {
    pub ctx: Option<subnet::rpc::context::Context<ValidatorStateClient>>,

    /// Represents persistent Vm state.
    pub state: Option<state::State>,
    /// Currently preferred block Id.
    pub preferred: ids::Id,

    /// Set "true" to indicate that the Vm has finished bootstrapping
    /// for the chain.
    pub bootstrapped: bool,
}

impl Default for VmState {
    fn default() -> Self {
        Self {
            ctx: None,
            state: None,
            preferred: ids::Id::empty(),
            bootstrapped: false,
        }
    }
}

/// Implements [`snowman.block.ChainVM`](https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/engine/snowman/block#ChainVM) interface.
#[derive(Clone)]
pub struct Vm {
    pub state: Arc<RwLock<VmState>>,

    pub app_sender: Option<AppSenderClient>,

    pub api_service: Option<RawApi>,

    pub api_context: Option<Context>,

    pub core_mempool: Option<Arc<RwLock<CoreMempool>>>,

    /// Channel to send messages to the snowman consensus engine.
    pub to_engine: Option<Arc<RwLock<Sender<snow::engine::common::message::Message>>>>,

    pub db: Option<Arc<RwLock<DbReaderWriter>>>,

    pub signer: Option<ValidatorSigner>,

    pub executor: Option<Arc<RwLock<BlockExecutor<AptosVM, Transaction>>>>,

    pub is_buiding_block: Arc<RwLock<bool>>,

}


impl Default for Vm

{
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(VmState::default())),
            app_sender: None,
            api_service: None,
            api_context: None,
            core_mempool: None,
            to_engine: None,
            signer: None,
            executor: None,
            db: None,
            is_buiding_block: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn is_bootstrapped(&self) -> bool {
        let vm_state = self.state.read().await;
        vm_state.bootstrapped
    }

    pub async fn get_transactions(&self, start: Option<U64>, limit: Option<u16>) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.0.get_transactions_raw(AcceptType::Json, start, limit).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_block_by_height(&self, height: u64, with_transactions: Option<bool>) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.5.get_block_by_height_raw(AcceptType::Json, height, with_transactions).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_block_by_version(&self, version: u64, with_transactions: Option<bool>) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.5.get_block_by_version_raw(AcceptType::Json, version, with_transactions).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_accounts_transactions(&self, args: RpcReq) -> RpcRes {
        let account = args.data.as_str();
        let api = self.api_service.as_ref().unwrap();
        let start = match args.start {
            None => None,
            Some(_) => Some(StateKeyWrapper::from_str(args.start.unwrap().as_str()).unwrap())
        };
        let ret = api.3.get_account_resources_raw(
            AcceptType::Json,
            Address::from_str(account).unwrap(),
            args.ledger_version,
            start,
            args.limit,
        ).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_account_resources(&self, args: RpcReq) -> RpcRes {
        let account = args.data.as_str();
        let api = self.api_service.as_ref().unwrap();
        let start = match args.start {
            None => None,
            Some(_) => Some(StateKeyWrapper::from_str(args.start.unwrap().as_str()).unwrap())
        };
        let ret = api.3.get_account_resources_raw(
            AcceptType::Json,
            Address::from_str(account).unwrap(),
            args.ledger_version,
            start,
            args.limit,
        ).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_account(&self, args: RpcReq) -> RpcRes {
        let account = args.data.as_str();
        let api = self.api_service.as_ref().unwrap();
        let ret = api.3.get_account_raw(AcceptType::Json,
                                        Address::from_str(account).unwrap(), args.ledger_version).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }
    pub async fn get_account_modules_state(&self, args: AccountStateArgs) -> RpcRes {
        let account = args.account.as_str();
        let module_name = args.resource.as_str();
        let module_name = IdentifierWrapper::from_str(module_name).unwrap();
        let api = self.api_service.as_ref().unwrap();
        let ret = api.4.get_account_module_raw(
            AcceptType::Json,
            Address::from_str(account).unwrap(),
            module_name, args.ledger_version).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_account_resources_state(&self, args: AccountStateArgs) -> RpcRes {
        let account = args.account.as_str();
        let resource = args.resource.as_str();
        let api = self.api_service.as_ref().unwrap();
        let ret = api.4.get_account_resource_raw(AcceptType::Json,
                                                 Address::from_str(account).unwrap(),
                                                 MoveStructTag::from_str(resource).unwrap(),
                                                 args.ledger_version).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_account_modules(&self, args: RpcReq) -> RpcRes {
        let account = args.data.as_str();
        let start = match args.start {
            None => None,
            Some(_) => Some(StateKeyWrapper::from_str(args.start.unwrap().as_str()).unwrap())
        };
        let api = self.api_service.as_ref().unwrap();
        let address = Address::from_str(account).unwrap();
        let ret = api.3.get_account_modules_raw(AcceptType::Json,
                                                address,
                                                args.ledger_version,
                                                start,
                                                args.limit).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_ledger_info(&self) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.2.get_ledger_info_raw(AcceptType::Json).await.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn view_function(&self, req: &str, ver: Option<U64>) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let req = serde_json::from_str::<ViewRequest>(req).unwrap();
        let ret = api.1.view_function_raw(
            AcceptType::Json,
            req,
            ver,
        ).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_transaction_by_hash(&self, h: &str) -> RpcRes {
        let h1 = HashValue::from_hex(h).unwrap();
        let hash = aptos_api_types::hash::HashValue::from(h1);
        let api = self.api_service.as_ref().unwrap();
        let ret = api.0.get_transaction_by_hash_raw(AcceptType::Json,
                                                    hash).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_transaction_by_version(&self, version: U64) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.0.get_transaction_by_version_raw(AcceptType::Json,
                                                       version).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn encode_submission(&self, data: &str) -> RpcRes {
        let service = self.api_service.as_ref().unwrap();
        let payload = serde_json::from_str::<EncodeSubmissionRequest>(data).unwrap();
        let ret =
            service.0.encode_submission_raw(AcceptType::Json, payload).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn submit_transaction(&self, data: Vec<u8>) -> RpcRes {
        log::info!("submit_transaction length {}",{data.len()});
        let service = self.api_service.as_ref().unwrap();
        let payload = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(data.clone()));
        let ret =
            service.0.submit_transaction_raw(AcceptType::Json, payload).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            SubmitTransactionResponse::Accepted(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        let signed_transaction: SignedTransaction =
            bcs::from_bytes_with_limit(&data,
                                       MAX_RECURSIVE_TYPES_ALLOWED as usize).unwrap();
        let sender = self.app_sender.as_ref().unwrap();
        sender.send_app_gossip(serde_json::to_vec(&signed_transaction.clone()).unwrap()).await.unwrap();
        self.add_pool(signed_transaction).await;
        self.notify_block_ready().await;
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn submit_transaction_batch(&self, data: Vec<u8>) -> RpcRes {
        log::info!("submit_transaction_batch length {}",{data.len()});
        let service = self.api_service.as_ref().unwrap();
        let payload = SubmitTransactionsBatchPost::Bcs(aptos_api::bcs_payload::Bcs(data.clone()));
        let ret = service.0.submit_transactions_batch_raw(AcceptType::Json,
                                                          payload).await;
        let ret = ret.unwrap();
        let mut failed_index = vec![];
        let header;
        let ret = match ret {
            SubmitTransactionsBatchResponse::Accepted(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
            SubmitTransactionsBatchResponse::AcceptedPartial(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        for x in &json.transaction_failures {
                            failed_index.push(x.transaction_index.clone());
                        }
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        let signed_transactions: Vec<SignedTransaction> =
            bcs::from_bytes(&data).unwrap();
        let sender = self.app_sender.as_ref().unwrap();
        let mut exist_count = 0;
        for (i, signed_transaction) in signed_transactions.iter().enumerate() {
            if !failed_index.contains(&i) {
                sender.send_app_gossip(serde_json::to_vec(signed_transaction).unwrap()).await.unwrap();
                self.add_pool(signed_transaction.clone()).await;
            } else {
                exist_count += 1;
            }
        }
        if exist_count > 0 {
            self.notify_block_ready().await;
        }
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }
    pub async fn get_table_item(&self, args: RpcTableReq) -> RpcRes {
        let account = args.query;
        let body = args.body;
        let payload = serde_json::from_str::<TableItemRequest>(body.as_str()).unwrap();
        let api = self.api_service.as_ref().unwrap();
        let ret = api.4.get_table_item_raw(
            AcceptType::Json,
            Address::from_str(account.as_str()).unwrap(),
            payload,
            args.ledger_version).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_raw_table_item(&self, args: RpcTableReq) -> RpcRes {
        let account = args.query;
        let body = args.body;
        let payload = serde_json::from_str::<RawTableItemRequest>(body.as_str()).unwrap();
        let api = self.api_service.as_ref().unwrap();
        let ret = api.4.get_raw_table_item_raw(
            AcceptType::Json,
            Address::from_str(account.as_str()).unwrap(),
            payload,
            args.ledger_version).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn get_events_by_creation_number(&self, args: RpcEventNumReq) -> RpcRes {
        let api = self.api_service.as_ref().unwrap();
        let ret = api.6.get_events_by_creation_number_raw(
            AcceptType::Json,
            Address::from_str(args.address.as_str()).unwrap(),
            args.creation_number,
            args.start, args.limit).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }
    pub async fn get_events_by_event_handle(&self, args: RpcEventHandleReq) -> RpcRes {
        let event_handle = MoveStructTag::from_str(args.event_handle.as_str()).unwrap();
        let field_name = IdentifierWrapper::from_str(args.field_name.as_str()).unwrap();
        let api = self.api_service.as_ref().unwrap();
        let ret = api.6.get_events_by_event_handle_raw(
            AcceptType::Json,
            Address::from_str(args.address.as_str()).unwrap(),
            event_handle,
            field_name, args.start, args.limit).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    async fn add_pool(&self, signed_transaction: SignedTransaction) {
        let mut core_pool = self.core_mempool.as_ref().unwrap().write().await;
        core_pool.add_txn(signed_transaction.clone(),
                          0,
                          signed_transaction.clone().sequence_number(),
                          TimelineState::NonQualified);
    }
    async fn get_pending_tx(&self, count: u64) -> Vec<SignedTransaction> {
        let core_pool = self.core_mempool.as_ref().unwrap().read().await;
        core_pool.get_batch(count,
                            1024 * 5 * 1000,
                            true,
                            true, vec![])
    }

    /// The logic of this function is to periodically check whether there is a block currently
    /// being constructed and whether there are pending transactions. If there is no block being
    /// constructed and the waiting time for unprocessed transactions has timed out, it allows
    /// the construction of a new block. If there is a block currently being constructed or the
    /// waiting time for unprocessed transactions has not timed out, it continues to wait.
    async fn check_pending_tx(&self) {
        // Clone the reference to `self` and create an Arc pointer to allow for multiple owners.
        let shared_self = Arc::new(self.clone());
        // Define the duration after which we will consider a check to have timed-out.
        let check_timeout_duration = Duration::from_secs(2);
        // Define the duration for which we will wait between each check.
        let check_duration = Duration::from_millis(500);
        // Spawn a new task to handle the checking logic asynchronously.
        tokio::task::spawn(async move {
            // Initialize a variable to keep track of the last time we checked for unprocessed transactions.
            let mut last_check_time = Instant::now();
            // Enter an infinite loop.
            loop {
                // Wait for the specified time interval before continuing.
                _ = tokio::time::sleep(check_duration).await;
                // Acquire a read lock on the shared `is_building_block` data to check if there are any blocks being built.
                let is_build = shared_self.is_buiding_block.read().await;
                if !*is_build { // If there is no building block...
                    // Check if the time elapsed since the last check is greater than the timeout duration.
                    if last_check_time.elapsed() > check_timeout_duration {
                        // If so, release the read lock and acquire a write lock to modify the shared data.
                        drop(is_build);
                        let mut is_build = shared_self.is_buiding_block.write().await;
                        // Set the `is_building_block` to `false` to indicate that a new block can now be built.
                        *is_build = false;
                    } else { // If the timeout duration has not yet been reached...
                        // Call the `get_pending_tx` function to acquire the list of unprocessed transactions.
                        let tx_arr = shared_self.get_pending_tx(1).await;
                        if !tx_arr.is_empty() { // If there are any unprocessed transactions...
                            // Notify the main thread that a block is ready to be built, and update the last check time.
                            shared_self.notify_block_ready().await;
                            last_check_time = Instant::now();
                        }
                    }
                } else { // If there is a building block...
                    // Update the last check time to prevent any timeouts during the block construction process.
                    last_check_time = Instant::now();
                }
            }
        });
    }


    async fn notify_block_ready(&self) {
        {
            let is_build = self.is_buiding_block.read().await;
            if *is_build {
                return;
            }
        }
        if let Some(to_engine) = &self.to_engine {
            let send_result = {
                let to_engine = to_engine.read().await;
                to_engine.send(PendingTxs).await
            };
            if send_result.is_ok() {
                let mut is_build = self.is_buiding_block.write().await;
                *is_build = true;
            } else {
                log::info!("send tx to_engine error ")
            }
        } else {
            log::info!("send tx to_engine error ")
        }
    }

    pub async fn simulate_transaction(&self, data: Vec<u8>) -> RpcRes {
        let service = self.api_service.as_ref().unwrap();
        let ret = service.0.simulate_transaction_raw(
            AcceptType::Json,
            Some(true),
            Some(false),
            Some(true),
            SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(data))).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn estimate_gas_price(&self) -> RpcRes {
        let service = self.api_service.as_ref().unwrap();
        let ret = service.0.estimate_gas_price_raw(
            AcceptType::Json).await;
        let ret = ret.unwrap();
        let header;
        let ret = match ret {
            BasicResponse::Ok(c, a, b, d, e, f, g, h, k) => {
                header = AptosHeader {
                    chain_id: a,
                    ledger_version: b,
                    ledger_oldest_version: d,
                    ledger_timestamp_usec: e,
                    epoch: f,
                    block_height: g,
                    oldest_block_height: h,
                    cursor: k,
                };
                match c {
                    AptosResponseContent::Json(json) => {
                        serde_json::to_string(&json.0).unwrap()
                    }
                    AptosResponseContent::Bcs(bytes) => {
                        format!("{}", hex::encode(bytes.0))
                    }
                }
            }
        };
        RpcRes { data: ret, header: serde_json::to_string(&header).unwrap() }
    }

    pub async fn facet_apt(&self, acc: Vec<u8>) -> RpcRes {
        let to = AccountAddress::from_bytes(acc).unwrap();
        let db = self.db.as_ref().unwrap().read().await;
        let mut core_account = self.get_core_account(&db).await;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_mint = core_account
            .sign_with_transaction_builder(
                tx_factory.mint(to, 10 * 100_000_000)
            );
        return self.submit_transaction(bcs::to_bytes(&tx_acc_mint).unwrap()).await;
    }

    pub async fn create_account(&self, key: &str) -> RpcRes {
        let to = Ed25519PublicKey::from_encoded_string(key).unwrap();
        let db = self.db.as_ref().unwrap().read().await;
        let mut core_account = self.get_core_account(&db).await;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_create = core_account
            .sign_with_transaction_builder(
                tx_factory.create_user_account(&to)
            );
        return self.submit_transaction(bcs::to_bytes(&tx_acc_create).unwrap()).await;
    }

    /// Sets the state of the Vm.
    /// # Errors
    /// Will fail if the `snow::State` is syncing
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
        let acc = aptos_test_root_address();
        let state_proof = db.reader.get_latest_ledger_info().unwrap();
        let current_version = state_proof.ledger_info().version();
        let db_state_view = db.reader.state_view_at_version(Some(current_version)).unwrap();
        let view = db_state_view.as_account_with_state_view(&acc);
        let av = view.get_account_resource().unwrap();
        let sn = av.unwrap().sequence_number();
        LocalAccount::new(
            aptos_test_root_address(),
            AccountKey::from_private_key(GENESIS_KEYPAIR.0.clone()),
            sn,
        )
    }
    pub async fn inner_build_block(&self, data: Vec<u8>, is_miner: bool) -> io::Result<Vec<u8>> {
        let executor = self.executor.as_ref().unwrap().read().await;
        let aptos_data = serde_json::from_slice::<AptosData>(&data).unwrap();
        let block_tx = serde_json::from_slice::<Vec<Transaction>>(&aptos_data.0).unwrap();
        let block_meta = block_tx.get(0).unwrap().try_as_block_metadata().unwrap();
        let block_id_now = block_meta.id();
        let block_id = aptos_data.1;

        if block_id_now.ne(&block_id) {
            return Err(Error::new(
                ErrorKind::Interrupted,
                "block format error",
            ));
        }
        let parent_block_id = aptos_data.2;
        let parent_block_id_now = executor.committed_block_id();
        if parent_block_id.ne(&parent_block_id_now) {
            return Err(Error::new(
                ErrorKind::Interrupted,
                "block error,maybe not sync ",
            ));
        }
        println!("------------inner_build_block {}----", block_id);
        let next_epoch = aptos_data.3;
        let ts = aptos_data.4;
        let output = executor
            .execute_block((block_id, block_tx.clone()), parent_block_id)
            .unwrap();
        let ledger_info = LedgerInfo::new(
            BlockInfo::new(
                next_epoch,
                0,
                block_id,
                output.root_hash(),
                output.version(),
                ts,
                output.epoch_state().clone(),
            ),
            HashValue::zero(),
        );
        let li;
        if is_miner {
            li = generate_ledger_info_with_sig(&[self.signer.as_ref().unwrap().clone()], ledger_info);
        } else {
            li = serde_json::from_slice::<LedgerInfoWithSignatures>(&aptos_data.5).unwrap();
        }
        executor.commit_blocks(vec![block_id], li.clone()).unwrap();
        let mut core_pool = self.core_mempool.as_ref().unwrap().write().await;
        for t in block_tx.iter() {
            match t {
                UserTransaction(t) => {
                    let sender = t.sender();
                    let sequence_number = t.sequence_number();
                    core_pool.commit_transaction(&AccountAddress::from(sender), sequence_number);
                }
                _ => {}
            }
        }
        let mut is_build = self.is_buiding_block.write().await;
        *is_build = false;
        if is_miner {
            Ok(serde_json::to_vec(&li).unwrap())
        } else {
            Ok(vec![])
        }
    }
    async fn init_aptos(&mut self) {
        let vm_state = self.state.write().await;
        let (genesis, validators) = test_genesis_change_set_and_validators(Some(1));
        let signer = ValidatorSigner::new(
            validators[0].data.owner_address,
            validators[0].consensus_key.clone(),
        );
        let db_path = vm_state.ctx.as_ref().unwrap().node_id.to_vec();
        self.signer = Some(signer.clone());
        let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
        let p = format!("{}/{}/{}",
                        dirs::home_dir().unwrap().to_str().unwrap(),
                        MOVE_DB_DIR,
                        hex::encode(db_path).as_str());
        let db;
        if !fs::metadata(p.clone().as_str()).is_ok() {
            fs::create_dir_all(p.as_str()).unwrap();
            db = DbReaderWriter::wrap(
                AptosDB::new_for_test(p.as_str()));
            let waypoint = generate_waypoint::<AptosVM>(&db.1, &genesis_txn).unwrap();
            maybe_bootstrap::<AptosVM>(&db.1, &genesis_txn, waypoint).unwrap();
        } else {
            db = DbReaderWriter::wrap(
                AptosDB::new_for_test(p.as_str()));
        }
        // BLOCK-STM
        // AptosVM::set_concurrency_level_once(2);
        self.db = Some(Arc::new(RwLock::new(db.1.clone())));
        let executor = BlockExecutor::new(db.1.clone());
        self.executor = Some(Arc::new(RwLock::new(executor)));

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
        self.check_pending_tx().await;
        tokio::task::spawn(async move {
            while let Some(request) = mempool_client_receiver.next().await {
                match request {
                    MempoolClientRequest::SubmitTransaction(_t, callback) => {
                        // accept all the transaction
                        let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
                        let status: SubmissionStatus = (ms, None);
                        callback.send(
                            Ok(status)
                        ).unwrap();
                    }
                    MempoolClientRequest::GetTransactionByHash(_, _) => {}
                }
            }
        });
    }
}

#[tonic::async_trait]
impl BatchedChainVm for Vm {
    type Block = Block;

    async fn get_ancestors(
        &self,
        _block_id: ids::Id,
        _max_block_num: i32,
        _max_block_size: i32,
        _max_block_retrival_time: Duration,
    ) -> io::Result<Vec<Bytes>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "get_ancestors not implemented",
        ))
    }

    async fn batched_parse_block(&self, _blocks: &[Vec<u8>]) -> io::Result<Vec<Self::Block>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "batched_parse_block not implemented",
        ))
    }
}

#[tonic::async_trait]
impl ChainVm for Vm
{
    type Block = Block;

    async fn build_block(
        &self,
    ) -> io::Result<<Self as ChainVm>::Block> {
        let vm_state = self.state.read().await;
        if let Some(state_b) = vm_state.state.as_ref() {
            let prnt_blk = state_b.get_block(&vm_state.preferred).await.unwrap();
            let unix_now = Utc::now().timestamp() as u64;
            let tx_arr = self.get_pending_tx(10000).await;
            println!("----build_block pool tx count-------{}------", tx_arr.clone().len());
            let executor = self.executor.as_ref().unwrap().read().await;
            let signer = self.signer.as_ref().unwrap();
            let db = self.db.as_ref().unwrap().read().await;
            let latest_ledger_info = db.reader.get_latest_ledger_info().unwrap();
            let next_epoch = latest_ledger_info.ledger_info().next_block_epoch();
            let block_id = HashValue::random();
            let block_meta = Transaction::BlockMetadata(BlockMetadata::new(
                block_id,
                next_epoch,
                0,
                signer.author(),
                vec![],
                vec![],
                unix_now,
            ));
            let mut txs = vec![];
            for tx in tx_arr.iter() {
                txs.push(UserTransaction(tx.clone()));
            }
            let mut block_tx: Vec<_> = vec![];
            block_tx.push(block_meta);
            block_tx.append(&mut txs);
            block_tx.push(Transaction::StateCheckpoint(HashValue::random()));
            let parent_block_id = executor.committed_block_id();
            let block_tx_bytes = serde_json::to_vec(&block_tx).unwrap();
            let mut data = AptosData(block_tx_bytes,
                                     block_id.clone(),
                                     parent_block_id,
                                     next_epoch,
                                     unix_now, vec![]);

            data.5 = self.inner_build_block(
                serde_json::to_vec(&data.clone()).unwrap(),
                true).await.unwrap();
            let mut block_ = Block::new(
                prnt_blk.id(),
                prnt_blk.height() + 1,
                unix_now,
                serde_json::to_vec(&data).unwrap(),
                choices::status::Status::Processing,
            ).unwrap();
            block_.set_state(state_b.clone());
            println!("--------vm_build_block------{}---", block_.id());
            block_.verify().await.unwrap();
            return Ok(block_);
        }
        Err(Error::new(
            ErrorKind::Other,
            "not implement",
        ))
    }

    async fn issue_tx(
        &self,
    ) -> io::Result<<Self as ChainVm>::Block> {
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
impl NetworkAppHandler for Vm
{
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

    async fn app_gossip(&self, _node_id: &ids::node::Id, msg: &[u8]) -> io::Result<()> {
        match serde_json::from_slice::<SignedTransaction>(msg) {
            Ok(s) => {
                self.add_pool(s).await;
            }
            Err(_) => {}
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl CrossChainAppHandler for Vm
{
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

impl AppHandler for Vm {}

#[tonic::async_trait]
impl Connector for Vm

{
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
impl Checkable for Vm
{
    async fn health_check(&self) -> io::Result<Vec<u8>> {
        Ok("200".as_bytes().to_vec())
    }
}

#[tonic::async_trait]
impl Getter for Vm
{
    type Block = Block;

    async fn get_block(
        &self,
        blk_id: ids::Id,
    ) -> io::Result<<Self as Getter>::Block> {
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let block = state.get_block(&blk_id).await?;
            return Ok(block);
        }
        Err(Error::new(ErrorKind::NotFound, "state manager not found"))
    }
}

#[tonic::async_trait]
impl Parser for Vm
{
    type Block = Block;
    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> io::Result<<Self as Parser>::Block> {
        let vm_state = self.state.read().await;
        if let Some(state) = vm_state.state.as_ref() {
            let mut new_block = Block::from_slice(bytes)?;
            new_block.set_status(choices::status::Status::Processing);
            let mut new_state = state.clone();
            new_state.set_vm(self.clone());
            new_block.set_state(new_state);
            return match state.get_block(&new_block.id()).await {
                Ok(prev) => {
                    Ok(prev)
                }
                Err(_) => {
                    Ok(new_block)
                }
            };
        }

        Err(Error::new(ErrorKind::NotFound, "state manager not found"))
    }
}

#[tonic::async_trait]
impl CommonVm for Vm
{
    type DatabaseManager = DatabaseManager;
    type AppSender = AppSenderClient;
    type ChainHandler = ChainHandler<ChainService>;
    type StaticHandler = StaticHandler;
    type ValidatorState = ValidatorStateClient;

    async fn initialize(
        &mut self,
        ctx: Option<subnet::rpc::context::Context<Self::ValidatorState>>,
        db_manager: Self::DatabaseManager,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: Sender<snow::engine::common::message::Message>,
        _fxs: &[snow::engine::common::vm::Fx],
        app_sender: Self::AppSender,
    ) -> io::Result<()> {
        let mut vm_state = self.state.write().await;
        vm_state.ctx = ctx.clone();
        let current = db_manager.current().await?;
        let state = state::State {
            db: Arc::new(RwLock::new(current.clone().db)),
            verified_blocks: Arc::new(RwLock::new(HashMap::new())),
            vm: None,
        };
        vm_state.state = Some(state.clone());
        self.to_engine = Some(Arc::new(RwLock::new(to_engine)));
        self.app_sender = Some(app_sender);
        drop(vm_state);

        self.init_aptos().await;
        let mut vm_state = self.state.write().await;
        let genesis = "hello world";
        let has_last_accepted = state.has_last_accepted_block().await?;
        if has_last_accepted {
            let last_accepted_blk_id = state.get_last_accepted_block_id().await?;
            vm_state.preferred = last_accepted_blk_id;
        } else {
            let genesis_bytes = genesis.as_bytes().to_vec();
            let data = AptosData(genesis_bytes.clone(),
                                 HashValue::zero(),
                                 HashValue::zero(),
                                 0,
                                 0,
                                 vec![]);
            let mut genesis_block = Block::new(
                ids::Id::empty(),
                0,
                0,
                serde_json::to_vec(&data).unwrap(),
                choices::status::Status::default(),
            ).unwrap();
            genesis_block.set_state(state.clone());
            genesis_block.accept().await?;

            let genesis_blk_id = genesis_block.id();
            vm_state.preferred = genesis_blk_id;
        }
        log::info!("successfully initialized Vm");
        Ok(())
    }

    async fn set_state(&self, snow_state: snow::State) -> io::Result<()> {
        self.set_state(snow_state).await
    }

    /// Called when the node is shutting down.
    async fn shutdown(&self) -> io::Result<()> {
        Ok(())
    }

    async fn version(&self) -> io::Result<String> {
        Ok(String::from(VERSION))
    }

    async fn create_static_handlers(
        &mut self,
    ) -> io::Result<HashMap<String, HttpHandler<Self::StaticHandler>>> {
        let handler = StaticHandler::new(StaticService::new());
        let mut handlers = HashMap::new();
        handlers.insert(
            "/static".to_string(),
            HttpHandler {
                lock_option: LockOptions::WriteLock,
                handler,
                server_addr: None,
            },
        );

        Ok(handlers)
    }

    async fn create_handlers(
        &mut self,
    ) -> io::Result<HashMap<String, HttpHandler<Self::ChainHandler>>> {
        let handler = ChainHandler::new(ChainService::new(self.clone()));
        let mut handlers = HashMap::new();
        handlers.insert(
            "/rpc".to_string(),
            HttpHandler {
                lock_option: LockOptions::WriteLock,
                handler,
                server_addr: None,
            },
        );

        Ok(handlers)
    }
}