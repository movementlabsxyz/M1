use avalanche_types::subnet::rpc::database::manager::{DatabaseManager, Manager};
use avalanche_types::subnet::rpc::health::Checkable;
use avalanche_types::subnet::rpc::snow::engine::common::appsender::client::AppSenderClient;
use avalanche_types::subnet::rpc::snow::engine::common::appsender::AppSender;
use avalanche_types::subnet::rpc::snow::engine::common::engine::{
    AppHandler, CrossChainAppHandler, NetworkAppHandler,
};
use avalanche_types::subnet::rpc::snow::engine::common::http_handler::{HttpHandler, LockOptions};
use avalanche_types::subnet::rpc::snow::engine::common::message::Message::PendingTxs;
use avalanche_types::subnet::rpc::snow::engine::common::vm::{CommonVm, Connector};
use avalanche_types::subnet::rpc::snow::validators::client::ValidatorStateClient;
use avalanche_types::subnet::rpc::snowman::block::{BatchedChainVm, ChainVm, Getter, Parser};
use avalanche_types::{
    choices, ids,
    subnet::{self, rpc::snow},
};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::{channel::mpsc as futures_mpsc, StreamExt};
use hex;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{
    collections::HashMap,
    fs,
    io::{self, Error, ErrorKind},
    sync::Arc,
};
use tokio::sync::{mpsc::Sender, RwLock};

use aptos_api::accept_type::{self, AcceptType};
use aptos_api::response::{AptosResponseContent, BasicResponse};
use aptos_api::transactions::{
    SubmitTransactionPost, SubmitTransactionResponse, SubmitTransactionsBatchPost,
    SubmitTransactionsBatchResponse,
};
use aptos_api::{get_raw_api_service, Context, RawApi};
use aptos_api_types::{
    Address, EncodeSubmissionRequest, IdentifierWrapper, MoveStructTag, RawTableItemRequest,
    StateKeyWrapper, TableItemRequest, ViewRequest, U64,
};
use aptos_config::config::NodeConfig;
use aptos_crypto::HashValue;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::BlockExecutorTrait;
use aptos_mempool::core_mempool::{CoreMempool, TimelineState};
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_sdk::rest_client::aptos_api_types::MAX_RECURSIVE_TYPES_ALLOWED;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::aptos_test_root_address;
use aptos_types::account_view::AccountView;
use aptos_types::block_executor::partitioner::{ExecutableBlock, ExecutableTransactions};
use aptos_types::block_info::BlockInfo;
use aptos_types::block_metadata::BlockMetadata;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo};
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::Transaction::UserTransaction;
use aptos_types::transaction::{SignedTransaction, Transaction, WriteSetPayload};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use aptos_vm_genesis::{test_genesis_change_set_and_validators, GENESIS_KEYPAIR};

use crate::api::chain_handlers::{
    AccountStateArgs, BlockArgs, ChainHandler, ChainService, GetTransactionByVersionArgs, PageArgs,
    RpcEventHandleReq, RpcEventNumReq, RpcReq, RpcRes, RpcTableReq,
};
use crate::api::static_handlers::{StaticHandler, StaticService};
use crate::{block::Block, state};
use anyhow::Context as AnyhowContext;
use aptos_types::account_config::AccountResource;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MOVE_DB_DIR: &str = ".move-chain-data";
pub fn get_db_name(suffix : &str) -> String {
    format!("{}-{}", MOVE_DB_DIR, suffix)
}


#[derive(Serialize, Deserialize, Clone)]
pub struct AptosData(
    pub Vec<u8>,   // block info
    pub HashValue, // block id
    pub HashValue,
    pub u64,
    pub u64,
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

    pub executor: Option<Arc<RwLock<BlockExecutor<AptosVM>>>>,

    pub build_status: Arc<RwLock<u8>>,
    // 0 done 1 building
    pub has_pending_tx: Arc<RwLock<bool>>,
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
            api_service: None,
            api_context: None,
            core_mempool: None,
            to_engine: None,
            signer: None,
            executor: None,
            db: None,
            build_status: Arc::new(RwLock::new(0)),
            has_pending_tx: Arc::new(RwLock::new(false)),
        }
    }
    #[allow(dead_code)]
    pub async fn is_bootstrapped(&self) -> bool {
        let vm_state = self.state.read().await;
        vm_state.bootstrapped
    }

    fn process_response<
        T: poem_openapi::types::ToJSON + Send + Sync + serde::Serialize,
        E: ToString + std::fmt::Debug,
    >(
        &self,
        ret: Result<BasicResponse<T>, E>,
    ) -> Result<RpcRes, anyhow::Error> {
        let mut ret_str = "".to_string();
        let mut error = None;
        let mut header_str = "".to_string();
        if ret.is_err() {
            error = match ret {
                Err(e) => Some(e.to_string()),
                _ => Some("Unknown error".to_string()),
            };
        } else {
            let ret = ret.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let header;
            ret_str = match ret {
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
                        AptosResponseContent::Json(json) => serde_json::to_string(&json.0)?,
                        AptosResponseContent::Bcs(bytes) => {
                            format!("{}", hex::encode(bytes.0))
                        },
                    }
                },
            };
            header_str = serde_json::to_string(&header)?;
        }

        Ok(RpcRes {
            data: ret_str,
            header: header_str,
            error,
        })

    }

   
    pub async fn get_transactions(&self, args: PageArgs) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .transactions_api
            .get_transactions_raw(accept, args.start, args.limit)
            .await;
        self.process_response(ret)
    }
    
    pub async fn get_block_by_height(&self, args: BlockArgs) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .blocks_api
            .get_block_by_height_raw(accept, args.height_or_version, args.with_transactions)
            .await;
        self.process_response(ret)
    }

    pub async fn get_block_by_version(&self, args: BlockArgs) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let api = self.api_service.as_ref().ok_or_else(
            || anyhow::anyhow!("API service not available"),
        )?;
        let ret = api
            .blocks_api
            .get_block_by_version_raw(accept, args.height_or_version, args.with_transactions)
            .await;
        self.process_response(ret)
    }
    

    // ^ refactored

    pub async fn get_accounts_transactions(&self, args: RpcReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.data.as_str();
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let start = args.start.as_deref()
            .map(U64::from_str)
            .transpose()
            .context("Failed to parse start parameter")?;
        let ret = api
            .transactions_api
            .get_accounts_transactions_raw(
                accept,
                Address::from_str(account).context("Invalid account address")?,
                start,
                args.limit,
            )
            .await;
        self.process_response(ret)
    }

    pub async fn get_account_resources(&self, args: RpcReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.data.as_str();
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let start = args.start
            .as_ref()
            .map(|s| StateKeyWrapper::from_str(s.as_str()))
            .transpose()
            .context("Failed to parse start parameter into StateKeyWrapper")?;
        let ret = api
            .accounts_api
            .get_account_resources_raw(
                accept,
                Address::from_str(account).context("Invalid account address")?,
                args.ledger_version,
                start,
                args.limit,
            )
            .await;
        self.process_response(ret)
    }
    
    // refactored

    pub async fn get_account(&self, args: RpcReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.data.as_str();
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .accounts_api
            .get_account_raw(
                accept,
                Address::from_str(account).context("Invalid account address")?,
                args.ledger_version,
            )
            .await;
        self.process_response(ret)
    }

    pub async fn get_account_modules_state(&self, args: AccountStateArgs) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.account.as_str();
        let module_name = IdentifierWrapper::from_str(args.resource.as_str()).context("Invalid module name")?;
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .state_api
            .get_account_module_raw(
                accept,
                Address::from_str(account).context("Invalid account address")?,
                module_name,
                args.ledger_version,
            )
            .await;
        self.process_response(ret)
    }

    pub async fn get_account_resources_state(&self, args: AccountStateArgs) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.account.as_str();
        let resource = args.resource.as_str();
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .state_api
            .get_account_resource_raw(
                accept,
                Address::from_str(account).context("Invalid account address")?,
                MoveStructTag::from_str(resource).context("Invalid resource tag")?,
                args.ledger_version,
            )
            .await;
        self.process_response(ret)
    }
    
    pub async fn get_account_modules(&self, args: RpcReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.data.as_str();
        let start = args.start
            .as_ref()
            .map(|s| StateKeyWrapper::from_str(s.as_str()))
            .transpose()
            .context("Failed to parse start parameter into StateKeyWrapper")?;
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let address = Address::from_str(account).context("Invalid account address")?;
        let ret = api
            .accounts_api
            .get_account_modules_raw(accept, address, args.ledger_version, start, args.limit)
            .await;
        self.process_response(ret)
    }

    // 

    pub async fn get_ledger_info(&self) -> Result<RpcRes, anyhow::Error> {
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api.index_api.get_ledger_info_raw(AcceptType::Json).await;
        self.process_response(ret)
    }
    
    pub async fn view_function(&self, args: RpcReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let req = serde_json::from_str::<ViewRequest>(args.data.as_str()).context("Failed to parse view function request")?;
        let ret = api
            .view_function_api
            .view_function_raw(accept, req, args.ledger_version)
            .await;
        self.process_response(ret)
    }
    
    pub async fn get_transaction_by_hash(&self, args: RpcReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let mut h = args.data.as_str();
        if h.starts_with("0x") {
            h = &h[2..];
        }
        let h1 = HashValue::from_hex(h).context("Failed to parse hash value")?;
        let hash = aptos_api_types::hash::HashValue::from(h1);
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api.transactions_api.get_transaction_by_hash_raw(accept, hash).await;
        self.process_response(ret)
    }
    
    pub async fn get_transaction_by_version(&self, args: GetTransactionByVersionArgs) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .transactions_api
            .get_transaction_by_version_raw(accept, args.version)
            .await;
        self.process_response(ret)
    }

    // refactor
    pub async fn encode_submission(&self, data: &str) -> Result<RpcRes, anyhow::Error> {
        let service = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let payload = serde_json::from_str::<EncodeSubmissionRequest>(data).context("Failed to parse encode submission request")?;
        let ret = service.transactions_api.encode_submission_raw(AcceptType::Json, payload).await;
        self.process_response(ret)
    }
    
    pub async fn submit_transaction(&self, data: Vec<u8>, accept: AcceptType) -> Result<RpcRes, anyhow::Error> {
        log::info!("submit_transaction length {}", data.len());
        let service = self.api_service.as_ref().ok_or_else(|| anyhow::Error::msg("API service not available"))?;
    
        let payload = SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(data.clone()));
        let response = service.transactions_api.submit_transaction_raw(accept, payload).await?;
    
        // Process the response
        let (ret_str, header_str) = match response {
            SubmitTransactionResponse::Accepted(content, chain_id, ledger_version, ledger_oldest_version, ledger_timestamp_usec, epoch, block_height, oldest_block_height, cursor) => {
                let header = AptosHeader {
                    chain_id,
                    ledger_version,
                    ledger_oldest_version,
                    ledger_timestamp_usec,
                    epoch,
                    block_height,
                    oldest_block_height,
                    cursor,
                };
                let content_str = match content {
                    AptosResponseContent::Json(json) => serde_json::to_string(&json.0)?,
                    AptosResponseContent::Bcs(bytes) => format!("{}", hex::encode(bytes.0)),
                };
                let header_str = serde_json::to_string(&header)?;
                (content_str, header_str)
            }
        };
    
        // Additional processing logic remains the same
        let signed_transaction: SignedTransaction = bcs::from_bytes_with_limit(&data, MAX_RECURSIVE_TYPES_ALLOWED as usize)?;
        let sender = self.app_sender.as_ref().ok_or_else(|| anyhow::Error::msg("App sender not available"))?;
        sender.send_app_gossip(serde_json::to_vec(&signed_transaction.clone())?).await?;
        self.add_pool(signed_transaction).await?;
        if data.len() >= 50 * 1024 {
            self.inner_build_block(self.build_block_data().await?).await?;
        } else {
            self.notify_block_ready().await;
        }
    
        Ok(RpcRes {
            data: ret_str,
            header: header_str,
            error: None,
        })
    }    
    
    pub async fn submit_transaction_batch(&self, data: Vec<u8>, accept: AcceptType) -> Result<RpcRes, anyhow::Error> {
        log::info!("submit_transaction_batch length {}", data.len());
        let service = self.api_service.as_ref().ok_or_else(|| anyhow::Error::msg("API service not available"))?;
        
        let payload = SubmitTransactionsBatchPost::Bcs(aptos_api::bcs_payload::Bcs(data.clone()));
        let response = service.transactions_api.submit_transactions_batch_raw(accept, payload)
            .await
            .context("Failed to submit transaction batch")?;
    
        // Directly handling the response parsing as before, but within the Result context
        let (ret_str, header_str, error) = match response {
            SubmitTransactionsBatchResponse::Accepted(content, chain_id, ledger_version, ledger_oldest_version, ledger_timestamp_usec, epoch, block_height, oldest_block_height, cursor) |
            SubmitTransactionsBatchResponse::AcceptedPartial(content, chain_id, ledger_version, ledger_oldest_version, ledger_timestamp_usec, epoch, block_height, oldest_block_height, cursor) => {
                let header = AptosHeader {
                    chain_id,
                    ledger_version,
                    ledger_oldest_version,
                    ledger_timestamp_usec,
                    epoch,
                    block_height,
                    oldest_block_height,
                    cursor,
                };
                let content_str = match content {
                    AptosResponseContent::Json(json) => serde_json::to_string(&json.0)?,
                    AptosResponseContent::Bcs(bytes) => format!("{}", hex::encode(bytes.0)),
                };
                let header_str = serde_json::to_string(&header)?;
                (content_str, header_str, None)
            },
        };
    
        // Since the original structure includes an error field, we handle it by returning an error string if applicable
        // In this context, error handling is managed above, so error here would be None.
        Ok(RpcRes {
            data: ret_str,
            header: header_str,
            error,
        })
    }
    

    //refactor

    pub async fn get_table_item(&self, args: RpcTableReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.query;
        let body = args.body;
        let payload = serde_json::from_str::<TableItemRequest>(body.as_str()).context("Failed to parse table item request")?;
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .state_api
            .get_table_item_raw(
                accept,
                Address::from_str(account.as_str()).context("Invalid account address")?,
                payload,
                args.ledger_version,
            )
            .await;
        self.process_response(ret)
    }
    
    pub async fn get_raw_table_item(&self, args: RpcTableReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let account = args.query;
        let body = args.body;
        let payload = serde_json::from_str::<RawTableItemRequest>(body.as_str()).context("Failed to parse raw table item request")?;
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .state_api
            .get_raw_table_item_raw(
                accept,
                Address::from_str(account.as_str()).context("Invalid account address")?,
                payload,
                args.ledger_version,
            )
            .await;
        self.process_response(ret)
    }
    
    pub async fn get_events_by_creation_number(&self, args: RpcEventNumReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .events_api
            .get_events_by_creation_number_raw(
                accept,
                Address::from_str(args.address.as_str()).context("Invalid address")?,
                args.creation_number,
                args.start,
                args.limit,
            )
            .await;
        self.process_response(ret)
    }
    
    pub async fn get_events_by_event_handle(&self, args: RpcEventHandleReq) -> Result<RpcRes, anyhow::Error> {
        let accept = if args.is_bcs_format.unwrap_or(false) {
            AcceptType::Bcs
        } else {
            AcceptType::Json
        };
        let event_handle = MoveStructTag::from_str(args.event_handle.as_str()).context("Invalid event handle")?;
        let field_name = IdentifierWrapper::from_str(args.field_name.as_str()).context("Invalid field name")?;
        let api = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = api
            .events_api
            .get_events_by_event_handle_raw(
                accept,
                Address::from_str(args.address.as_str()).context("Invalid address")?,
                event_handle,
                field_name,
                args.start,
                args.limit,
            )
            .await;
        self.process_response(ret)
    }

    // refactor

    pub async fn simulate_transaction(&self, data: Vec<u8>, accept: AcceptType) -> Result<RpcRes, anyhow::Error> {
        let service = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = service
            .transactions_api
            .simulate_transaction_raw(
                accept,
                Some(true),
                Some(false),
                Some(true),
                SubmitTransactionPost::Bcs(aptos_api::bcs_payload::Bcs(data)),
            )
            .await;
        self.process_response(ret)
    }
    
    pub async fn estimate_gas_price(&self) -> Result<RpcRes, anyhow::Error> {
        let service = self.api_service.as_ref().ok_or_else(|| anyhow::anyhow!("API service not available"))?;
        let ret = service.transactions_api.estimate_gas_price_raw(AcceptType::Json).await;
        self.process_response(ret)
    }
    
    async fn add_pool(&self, signed_transaction: SignedTransaction) -> Result<(), anyhow::Error> {
        let mut core_pool = self.core_mempool.as_ref().ok_or_else(|| anyhow::anyhow!("Core mempool not available"))?.write().await;
        core_pool.add_txn(
            signed_transaction.clone(),
            0,
            signed_transaction.sequence_number(),
            TimelineState::NonQualified,
            true,
        );
        Ok(())
    }
    
    async fn get_pending_tx(&self, count: u64) -> Result<Vec<SignedTransaction>, anyhow::Error> {
        let core_pool = self.core_mempool.as_ref().ok_or_else(|| anyhow::anyhow!("Core mempool not available"))?.read().await;
        Ok(core_pool.get_batch(count, 1024 * 5 * 1000, true, true, vec![]))
    }
    

    async fn check_pending_tx(&self) -> Result<(), anyhow::Error> {
        let shared_self = Arc::new(self.clone());
        let check_duration = Duration::from_millis(2000);
        tokio::spawn(async move {
            let mut last_check_build_time = get_current_time_seconds();
            loop {
                tokio::time::sleep(check_duration).await;
                let build_status_result = shared_self.build_status.try_read();
                match build_status_result {
                    Ok(build_status) => {
                        if *build_status == 0 {
                            let has_pending_tx = shared_self.has_pending_tx.try_read();
                            match has_pending_tx {
                                Ok(has_pending) => {
                                    if *has_pending {
                                        shared_self.update_pending_tx_flag(false).await;
                                        shared_self.notify_block_ready2().await;
                                    }
                                },
                                Err(e) => log::error!("Failed to read has_pending_tx: {}", e),
                            }
                        }
                    },
                    Err(e) => log::error!("Failed to read build status: {}", e),
                }

                if get_current_time_seconds() - last_check_build_time > 120 {
                    shared_self.update_build_block_status(0).await; 
                    last_check_build_time = get_current_time_seconds();
                }
            }
        });
        Ok(())
    }


        // Updates the build block status safely within an asynchronous context.
    // This function already handles its main purpose well. Consider logging or acting on error conditions if needed.
    async fn update_build_block_status(&self, s: u8) {
        let mut status = self.build_status.write().await;
        if *status != s {
            *status = s;
        }
    }

    // Toggles the pending transaction flag, indicating whether there are transactions pending processing.
    // Similar to `update_build_block_status`, it's straightforward and handles its purpose well.
    async fn update_pending_tx_flag(&self, n: bool) {
        let mut tx = self.has_pending_tx.write().await;
        if *tx != n {
            *tx = n;
        }
    }

    // Notifies that the block is ready to be processed, attempting to send a message to the engine.
    // Added error handling or logging for the send operation could be beneficial.
    async fn notify_block_ready2(&self) {
        if let Some(to_engine) = &self.to_engine {
            match to_engine.read().await.send(PendingTxs).await {
                Ok(_) => {
                    self.update_build_block_status(1).await;
                    log::info!("notify_block_ready:success");
                },
                Err(e) => log::error!("send tx to_engine error: {}", e),
            }
        } else {
            log::error!("to_engine is None, cannot send tx");
        }
    }

    // Checks the current block status and decides whether to notify that the block is ready.
    // This function primarily orchestrates the condition checking and delegating.
    async fn notify_block_ready(&self) {
        let status = *self.build_status.read().await;
        let tx = *self.has_pending_tx.read().await;

        match status {
            1 if !tx => self.update_pending_tx_flag(true).await, // If building and no transactions are pending, flag is set.
            0 => self.notify_block_ready2().await, // If not building, proceed to notify.
            _ => log::info!("notify_block_ready ignore or unhandled status: {}", status),
        }
    }


    pub async fn faucet_apt(&self, acc: Vec<u8>, accept: AcceptType) -> Result<RpcRes, anyhow::Error> {
        let to = AccountAddress::from_bytes(acc).context("Failed to convert account address")?;
        let db = self.db.as_ref().ok_or_else(|| anyhow::anyhow!("Database reference not found"))?.read().await;
        let core_account = self.get_core_account(&db).await?;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_mint = core_account.sign_with_transaction_builder(tx_factory.mint(to, 10 * 100_000_000));
        self.submit_transaction(bcs::to_bytes(&tx_acc_mint)?, accept).await
    }

   
    
    pub async fn faucet_with_cli(&self, acc: Vec<u8>) -> Result<RpcRes, anyhow::Error> {
        // ! the below creates some kind of race condition
        /*match self.view_account(acc.clone()).await? {
            Some(_) => {},
            None =>{
                self.create_account(acc.clone(), AcceptType::Bcs).await?;
            }
        };*/
        let to = AccountAddress::from_bytes(acc).context("Failed to convert account address")?;
        let db = self.db.as_ref().ok_or_else(|| anyhow::anyhow!("Database reference not found"))?.read().await;
        let core_account = self.get_core_account(&db).await?;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_mint = core_account.sign_with_transaction_builder(tx_factory.mint(to, 10 * 100_000_000));
        let mut res = self.submit_transaction(bcs::to_bytes(&tx_acc_mint)?, AcceptType::Bcs).await?;
        let txs = vec![tx_acc_mint];
        res.data = hex::encode(bcs::to_bytes(&txs)?);
        Ok(res)
    }
    
    pub async fn create_account(&self, acc: Vec<u8>, accept: AcceptType) -> Result<RpcRes, anyhow::Error> {
        let to = AccountAddress::from_bytes(acc).context("Failed to convert account address")?;
        let db = self.db.as_ref().ok_or_else(|| anyhow::anyhow!("Database reference not found"))?.read().await;
        let core_account = self.get_core_account(&db).await?;
        let tx_factory = TransactionFactory::new(ChainId::test());
        let tx_acc_create = core_account.sign_with_transaction_builder(tx_factory.create_account(to));
        self.submit_transaction(bcs::to_bytes(&tx_acc_create)?, accept).await
    }
    
    pub async fn set_state(&self, snow_state: snow::State) -> Result<(), anyhow::Error> {
        let mut vm_state = self.state.write().await;
        match snow_state {
            snow::State::Initializing => {
                log::info!("set_state: initializing");
                vm_state.bootstrapped = false;
                Ok(())
            },
            snow::State::StateSyncing => {
                log::info!("set_state: state syncing");
                Err(anyhow::anyhow!("state sync is not supported"))
            },
            snow::State::Bootstrapping => {
                log::info!("set_state: bootstrapping");
                vm_state.bootstrapped = false;
                Ok(())
            },
            snow::State::NormalOp => {
                log::info!("set_state: normal op");
                vm_state.bootstrapped = true;
                Ok(())
            },
        }
    }    

    pub async fn set_preference(&self, id: ids::Id) -> Result<(), anyhow::Error> {
        let mut vm_state = self.state.write().await;
        vm_state.preferred = id;
        Ok(())
    }
    
    pub async fn last_accepted(&self) -> Result<ids::Id, anyhow::Error> {
        let vm_state = self.state.read().await;
        if let Some(state) = &vm_state.state {
            let blk_id = state.get_last_accepted_block_id().await.context("Failed to get last accepted block ID")?;
            Ok(blk_id)
        } else {
            Err(anyhow::anyhow!("State manager not found").into())
        }
    }

    pub async fn view_account(&self, acc: Vec<u8>) -> Result<Option<AccountResource>, anyhow::Error> {
        let db = self.db.as_ref().ok_or_else(|| anyhow::anyhow!("Database reference not found"))?.read().await;
        let state_proof = db.reader.get_latest_ledger_info().context("Failed to get latest ledger info")?;
        let current_version = state_proof.ledger_info().version();
        let db_state_view = db
            .reader
            .state_view_at_version(Some(current_version))
            .context("Failed to get DB state view at version")?;
        let account_address = AccountAddress::from_bytes(acc.as_slice()).context("Failed to convert account address")?;
        let view = db_state_view.
        as_account_with_state_view(&account_address);
        let av = view.get_account_resource()?;
        Ok(av)
    }
    
    pub async fn get_core_account(&self, db: &DbReaderWriter) -> Result<LocalAccount, anyhow::Error> {
        let acc = aptos_test_root_address();
        let state_proof = db.reader.get_latest_ledger_info().context("Failed to get latest ledger info")?;
        let current_version = state_proof.ledger_info().version();
        let db_state_view = db
            .reader
            .state_view_at_version(Some(current_version))
            .context("Failed to get DB state view at version")?;
        let view = db_state_view.as_account_with_state_view(&acc);
        let av = view.get_account_resource()?
        .ok_or_else(|| anyhow::Error::msg("Account resource not found"))
        .context("Failed to get account resource")?;
        let sn = av.sequence_number();
        Ok(LocalAccount::new(
            aptos_test_root_address(),
            AccountKey::from_private_key(GENESIS_KEYPAIR.0.clone()),
            sn,
        ))
    }
    
    pub async fn inner_build_block(&self, data: Vec<u8>) -> Result<(), anyhow::Error> {

        // get executor and metadata
        let executor = self.executor.as_ref().ok_or_else(|| anyhow::anyhow!("Executor not available"))?.read().await;
        let aptos_data = serde_json::from_slice::<AptosData>(&data).context("Failed to parse AptosData from bytes")?;
        let block_tx = serde_json::from_slice::<Vec<Transaction>>(&aptos_data.0).context("Failed to parse transactions from AptosData")?;
        let block_meta = block_tx.get(0).ok_or_else(|| anyhow::anyhow!("Block metadata not found in transactions"))?.try_as_block_metadata().context("Failed to convert transaction to block metadata")?;

        // execute block
        let block_id = block_meta.id();
        let parent_block_id = executor.committed_block_id();
        let next_epoch = aptos_data.3;
        let ts = aptos_data.4;
        let output = executor.execute_block(
            ExecutableBlock::new(block_id, ExecutableTransactions::Unsharded(block_tx.clone())),
            parent_block_id,
            None,
        ).context("Failed to execute block")?;

        // commit block
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
        let signer = self.signer.as_ref().ok_or_else(|| anyhow::anyhow!("Signer not available"))?;
        let li = generate_ledger_info_with_sig(
            &[signer.clone()],
            ledger_info,
        );
        executor.commit_blocks(vec![block_id], li.clone())?;

        // add
        let mut core_pool = self.core_mempool.as_ref().ok_or_else(|| anyhow::anyhow!("Core mempool not available"))?.write().await;
        for t in block_tx.iter() {
            if let UserTransaction(t) = t {
                let sender = t.sender();
                let sequence_number = t.sequence_number();
                core_pool.commit_transaction(&AccountAddress::from(sender), sequence_number);
            }
        }
        self.update_build_block_status(0).await;
        Ok(())
    }



    async fn init_aptos(&mut self, uuid: &str) -> Result<(), anyhow::Error> {
        let db_name = get_db_name(uuid);
        let (genesis, validators) = test_genesis_change_set_and_validators(Some(1));
        let signer = ValidatorSigner::new(validators[0].data.owner_address, validators[0].consensus_key.clone());
        self.signer = Some(signer);

        let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
        let p = format!("{}/{}", dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Home directory not found"))?.to_str().ok_or_else(|| anyhow::anyhow!("Failed to convert home directory to string"))?, db_name);

        if fs::metadata(&p).is_err() {
            fs::create_dir_all(&p).context("Failed to create directory")?;
        }

        let db = DbReaderWriter::wrap(AptosDB::new_for_test(&p));
        let waypoint = generate_waypoint::<AptosVM>(&db.1, &genesis_txn).context("Failed to generate waypoint")?;
        maybe_bootstrap::<AptosVM>(&db.1, &genesis_txn, waypoint).context("Failed to bootstrap DB")?;

        self.db = Some(Arc::new(RwLock::new(db.1.clone())));
        let executor = BlockExecutor::new(db.1.clone());
        self.executor = Some(Arc::new(RwLock::new(executor)));

        let (mempool_client_sender, mut mempool_client_receiver) = futures_mpsc::channel::<MempoolClientRequest>(10);
        let sender = MempoolClientSender::from(mempool_client_sender);
        let node_config = NodeConfig::default();
        let context = Context::new(ChainId::test(), db.1.reader.clone(), sender, node_config.clone());
        self.api_context = Some(context.clone());
        let service = get_raw_api_service(Arc::new(context));
        self.api_service = Some(service);
        self.core_mempool = Some(Arc::new(RwLock::new(CoreMempool::new(&node_config))));
        self.check_pending_tx().await?;

        tokio::task::spawn(async move {
            while let Some(request) = mempool_client_receiver.next().await {
                match request {
                    MempoolClientRequest::SubmitTransaction(_t, callback) => {
                        // accept all the transaction
                        let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
                        let status: SubmissionStatus = (ms, None);
                        callback.send(Ok(status)).unwrap();
                    },
                    MempoolClientRequest::GetTransactionByHash(_, _) => {},
                }
            }
        });

        Ok(())
    }

    async fn build_block_data(&self) -> Result<Vec<u8>, anyhow::Error> {
        let unix_now_micro = Utc::now().timestamp_micros() as u64;
        let tx_arr = self.get_pending_tx(500).await?;
        log::info!("build_block pool tx count {}", tx_arr.len());
        let executor = self.executor.as_ref().ok_or_else(|| anyhow::anyhow!("Executor not available"))?.read().await;
        let signer = self.signer.as_ref().ok_or_else(|| anyhow::anyhow!("Signer not available"))?;
        let db = self.db.as_ref().ok_or_else(|| anyhow::anyhow!("DB not available"))?.read().await;
        let latest_ledger_info = db.reader.get_latest_ledger_info().context("Failed to get latest ledger info")?;
        let next_epoch = latest_ledger_info.ledger_info().next_block_epoch();
        let block_id = HashValue::random();
        let block_meta = Transaction::BlockMetadata(BlockMetadata::new(block_id, next_epoch, 0, signer.author(), vec![], vec![], unix_now_micro));
    
        let mut txs: Vec<Transaction> = tx_arr.into_iter().map(UserTransaction).collect();
        txs.insert(0, block_meta);
        txs.push(Transaction::StateCheckpoint(HashValue::random()));
    
        let parent_block_id = executor.committed_block_id();
        let data = AptosData(serde_json::to_vec(&txs)?, block_id, parent_block_id, next_epoch, unix_now_micro);
    
        serde_json::to_vec(&data).context("Failed to serialize block data")
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
impl ChainVm for Vm {
    type Block = Block;

    async fn build_block(&self) -> io::Result<<Self as ChainVm>::Block> {
        let vm_state = self.state.read().await;
        if let Some(state_b) = vm_state.state.as_ref() {
            let prnt_blk = state_b.get_block(&vm_state.preferred).await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get parent block: {}", e)))?;
            let unix_now = Utc::now().timestamp() as u64;

            let data = self.build_block_data().await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to build block data: {}", e)))?;
            let mut block_ = Block::new(prnt_blk.id(), prnt_blk.height() + 1, unix_now, data, choices::status::Status::Processing)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to create new block: {}", e)))?;
            block_.set_state(state_b.clone());
            block_.verify().await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Block verification failed: {}", e)))?;
            Ok(block_)
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "VM state not initialized"))
        }
    }

    async fn issue_tx(&self) ->  io::Result<<Self as ChainVm>::Block> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "issue_tx not implemented"))
    }

    async fn set_preference(&self, id: ids::Id) -> io::Result<()> {
        self.set_preference(id).await.map_err(
            |e| io::Error::new(io::ErrorKind::Other, format!("Failed to set preference: {}", e))
        )
    }

    async fn last_accepted(&self) -> io::Result<ids::Id> {
        self.last_accepted().await.map_err(
            |e| io::Error::new(io::ErrorKind::Other, format!("Failed to get last accepted block: {}", e))
        )
    }

    async fn verify_height_index(&self) -> io::Result<()> {
        Ok(())
    }

    async fn get_block_id_at_height(&self, _height: u64) -> io::Result<ids::Id> {
        Err(io::Error::new(io::ErrorKind::NotFound, "block id not found"))
    }

    async fn state_sync_enabled(&self) -> io::Result<bool> {
        Ok(false)
    }
}

#[tonic::async_trait]
impl NetworkAppHandler for Vm {
    async fn app_request(&self, _node_id: &ids::node::Id, _request_id: u32, _deadline: DateTime<Utc>, _request: &[u8]) -> io::Result<()> {
        Ok(())
    }

    async fn app_request_failed(&self, _node_id: &ids::node::Id, _request_id: u32) -> io::Result<()> {
        Ok(())
    }

    async fn app_response(&self, _node_id: &ids::node::Id, _request_id: u32, _response: &[u8]) -> io::Result<()> {
        Ok(())
    }

    async fn app_gossip(&self, _node_id: &ids::node::Id, msg: &[u8]) -> io::Result<()> {
        if let Ok(s) = serde_json::from_slice::<SignedTransaction>(msg) {
            self.add_pool(s).await.map_err(
                |e| io::Error::new(io::ErrorKind::Other, format!("Failed to add transaction to pool: {}", e))
            )?;
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl CrossChainAppHandler for Vm {
    async fn cross_chain_app_request(&self, _chain_id: &ids::Id, _request_id: u32, _deadline: DateTime<Utc>, _request: &[u8]) -> io::Result<()> {
        Ok(())
    }

    async fn cross_chain_app_request_failed(&self, _chain_id: &ids::Id, _request_id: u32) -> io::Result<()> {
        Ok(())
    }

    async fn cross_chain_app_response(&self, _chain_id: &ids::Id, _request_id: u32, _response: &[u8]) -> io::Result<()> {
        Ok(())
    }
}

impl AppHandler for Vm {}

#[tonic::async_trait]
impl Connector for Vm {
    async fn connected(&self, _id: &ids::node::Id) -> io::Result<()> {
        Ok(())
    }

    async fn disconnected(&self, _id: &ids::node::Id) -> io::Result<()> {
        Ok(())
    }
}

#[tonic::async_trait]
impl Checkable for Vm {
    async fn health_check(&self) -> io::Result<Vec<u8>> {
        Ok("200".as_bytes().to_vec())
    }
}

#[tonic::async_trait]
impl Getter for Vm {
    type Block = Block;

    async fn get_block(&self, blk_id: ids::Id) -> io::Result<Self::Block> {
        let vm_state = self.state.read().await;
        let state = vm_state.state
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "VM state not initialized"))?;
    
        // Now that we have confirmed `state` is available, proceed with the async operation
        let mut block = state.get_block(&blk_id).await
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "Block not found"))?;
        
        let mut new_state = state.clone();
        new_state.set_vm(self.clone());
        block.set_state(new_state);
        
        Ok(block)
    }
    
}

#[tonic::async_trait]
impl Parser for Vm {
    type Block = Block;

    async fn parse_block(&self, bytes: &[u8]) -> io::Result<Self::Block> {
        let vm_state = self.state.read().await;
        if let Some(state) = vm_state.state.as_ref() {
            let mut new_block = Block::from_slice(bytes)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            new_block.set_status(choices::status::Status::Processing);
            let mut new_state = state.clone();
            new_state.set_vm(self.clone());
            new_block.set_state(new_state);
            state.get_block(&new_block.id()).await.or(Ok(new_block))
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "state manager not found"))
        }
    }
}


#[tonic::async_trait]
impl CommonVm for Vm {
    type DatabaseManager = DatabaseManager;
    type AppSender = AppSenderClient;
    type ChainHandler = ChainHandler<ChainService>;
    type StaticHandler = StaticHandler;
    type ValidatorState = ValidatorStateClient;
    
    async fn initialize(
        &mut self,
        ctx: Option<subnet::rpc::context::Context<Self::ValidatorState>>,
        db_manager: Self::DatabaseManager,
        _genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: Sender<snow::engine::common::message::Message>,
        _fxs: &[snow::engine::common::vm::Fx],
        app_sender: Self::AppSender,
    ) -> io::Result<()> {
        let uuid = std::env::var("M1_ID").unwrap_or(uuid::Uuid::new_v4().to_string());
        log::info!("Initializing M1 Vm {}", uuid);

        let state = {
            let mut vm_state = self.state.write().await;
            vm_state.ctx = ctx;
    
            let current = db_manager.current().await.map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get current DB manager: {}", e)))?;
            let state = state::State {
                db: Arc::new(RwLock::new(current.db)),
                verified_blocks: Arc::new(RwLock::new(HashMap::new())),
                vm: None,
            };
            vm_state.state = Some(state.clone());
            self.to_engine = Some(Arc::new(RwLock::new(to_engine)));
            self.app_sender = Some(app_sender);
            state
        };
       
        if let Err(e) = self.init_aptos(&uuid).await {
            return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to initialize Aptos: {}", e)));
        }

        let mut vm_state = self.state.write().await;
        let genesis = "hello world";
        let has_last_accepted = state.has_last_accepted_block().await?;
        if has_last_accepted {
            let last_accepted_blk_id = state.get_last_accepted_block_id().await?;
            vm_state.preferred = last_accepted_blk_id;
        } else {
            let genesis_bytes = genesis.as_bytes().to_vec();
            let data = AptosData(
                genesis_bytes.clone(),
                HashValue::zero(),
                HashValue::zero(),
                0,
                0,
            );
            let mut genesis_block = Block::new(
                ids::Id::empty(),
                0,
                0,
                serde_json::to_vec(&data)?,
                choices::status::Status::default(),
            )?;
            genesis_block.set_state(state.clone());
            genesis_block.accept().await?;

            let genesis_blk_id = genesis_block.id();
            vm_state.preferred = genesis_blk_id;
        }
        log::info!("successfully initialized Vm");

        // Post-initialization logic, such as setting preferred block id, is already handled within init_aptos
        log::info!("Successfully initialized Vm");
        Ok(())
    }

    async fn set_state(&self, snow_state: snow::State) -> io::Result<()> {
        self.set_state(snow_state).await.map_err(
            |e| io::Error::new(io::ErrorKind::Other, format!("Failed to set state: {}", e))
        )
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

fn get_current_time_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get timestamp")
        .as_secs()
}
