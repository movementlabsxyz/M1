use std::io;
use std::marker::PhantomData;

use aptos_api::accept_type::AcceptType;
use aptos_api_types::U64;
use avalanche_types::proto::http::Element;
use avalanche_types::subnet::rpc::http::handle::Handle;
use bytes::Bytes;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, IoHandler, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use crate::api::de_request;
use crate::util::HexParser;
use crate::vm::Vm;

#[rpc]
pub trait Rpc {
    /*******************************TRANSACTION START***************************************/
    #[rpc(name = "getTransactions", alias("aptosvm.getTransactions"))]
    fn get_transactions(&self, args: PageArgs) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "submitTransaction", alias("aptosvm.submitTransaction"))]
    fn submit_transaction(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "submitTransactionBatch",
        alias("aptosvm.submitTransactionBatch")
    )]
    fn submit_transaction_batch(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getTransactionByHash", alias("aptosvm.getTransactionByHash"))]
    fn get_transaction_by_hash(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "getTransactionByVersion",
        alias("aptosvm.getTransactionByVersion")
    )]
    fn get_transaction_by_version(
        &self,
        args: GetTransactionByVersionArgs,
    ) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "getAccountsTransactions",
        alias("aptosvm.getAccountsTransactions")
    )]
    fn get_accounts_transactions(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "simulateTransaction", alias("aptosvm.simulateTransaction"))]
    fn simulate_transaction(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "encodeSubmission", alias("aptosvm.encodeSubmission"))]
    fn encode_submission(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "estimateGasPrice", alias("aptosvm.estimateGasPrice"))]
    fn estimate_gas_price(&self) -> BoxFuture<Result<RpcRes>>;
    /*******************************TRANSACTION END***************************************/

    /*******************************HELPER API START***************************************/
    #[rpc(name = "faucet", alias("aptosvm.faucet"))]
    fn faucet_apt(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "faucetWithCli")]
    fn faucet_with_cli(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "createAccount", alias("aptosvm.createAccount"))]
    fn create_account(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    /*******************************HELPER API END***************************************/

    /******************************* ACCOUNT START ***************************************/

    #[rpc(name = "getAccount", alias("aptosvm.getAccount"))]
    fn get_account(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getAccountResources", alias("aptosvm.getAccountResources"))]
    fn get_account_resources(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getAccountModules", alias("aptosvm.getAccountModules"))]
    fn get_account_modules(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "getAccountResourcesState",
        alias("aptosvm.getAccountResourcesState")
    )]
    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "getAccountModulesState",
        alias("aptosvm.getAccountModulesState")
    )]
    fn get_account_modules_state(&self, args: AccountStateArgs) -> BoxFuture<Result<RpcRes>>;
    /******************************* ACCOUNT END ***************************************/

    /*******************************BLOCK START***************************************/
    #[rpc(name = "getBlockByHeight", alias("aptosvm.getBlockByHeight"))]
    fn get_block_by_height(&self, args: BlockArgs) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getBlockByVersion", alias("aptosvm.getBlockByVersion"))]
    fn get_block_by_version(&self, args: BlockArgs) -> BoxFuture<Result<RpcRes>>;
    /*******************************BLOCK END***************************************/

    #[rpc(name = "viewFunction", alias("aptosvm.viewFunction"))]
    fn view_function(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getTableItem", alias("aptosvm.getTableItem"))]
    fn get_table_item(&self, args: RpcTableReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getRawTableItem", alias("aptosvm.getRawTableItem"))]
    fn get_raw_table_item(&self, args: RpcTableReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "getEventsByCreationNumber",
        alias("aptosvm.getEventsByCreationNumber")
    )]
    fn get_events_by_creation_number(&self, args: RpcEventNumReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(
        name = "getEventsByEventHandle",
        alias("aptosvm.getEventsByEventHandle")
    )]
    fn get_events_by_event_handle(&self, args: RpcEventHandleReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getLedgerInfo", alias("aptosvm.getLedgerInfo"))]
    fn get_ledger_info(&self) -> BoxFuture<Result<RpcRes>>;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTableItemArgs {
    pub table_handle: String,
    pub key_type: String,
    pub value_type: String,
    pub key: String,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcReq {
    pub data: String,
    pub ledger_version: Option<U64>,
    pub start: Option<String>,
    pub limit: Option<u16>,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcRes {
    pub data: String,
    pub header: String,
    pub error: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcTableReq {
    pub query: String,
    pub body: String,
    pub ledger_version: Option<U64>,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcEventNumReq {
    pub address: String,
    pub creation_number: U64,
    pub start: Option<U64>,
    pub limit: Option<u16>,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcEventHandleReq {
    pub start: Option<U64>,
    pub limit: Option<u16>,
    pub address: String,
    pub event_handle: String,
    pub field_name: String,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BlockArgs {
    pub height_or_version: u64,
    pub with_transactions: Option<bool>,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTransactionByVersionArgs {
    pub version: U64,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountStateArgs {
    pub account: String,
    pub resource: String,
    pub ledger_version: Option<U64>,
    pub is_bcs_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PageArgs {
    pub start: Option<U64>,
    pub limit: Option<u16>,
    pub is_bcs_format: Option<bool>,
}

#[derive(Clone)]
pub struct ChainService {
    pub vm: Vm,
}

impl ChainService {
    pub fn new(vm: Vm) -> Self {
        Self { vm }
    }
}

impl Rpc for ChainService {
    fn get_transactions(&self, args: PageArgs) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_transactions(args).await;
            return Ok(ret);
        })
    }

    fn submit_transaction(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        log::debug!("submit_transaction called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let accept = if args.is_bcs_format.unwrap_or(false) {
                AcceptType::Bcs
            } else {
                AcceptType::Json
            };
            let r = vm
                .submit_transaction(hex::decode(args.data).unwrap(), accept)
                .await;
            Ok(r)
        })
    }

    fn submit_transaction_batch(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let accept = if args.is_bcs_format.unwrap_or(false) {
                AcceptType::Bcs
            } else {
                AcceptType::Json
            };
            let r = vm
                .submit_transaction_batch(hex::decode(args.data).unwrap(), accept)
                .await;
            Ok(r)
        })
    }

    fn get_transaction_by_hash(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_transaction_by_hash(args).await;
            return Ok(ret);
        })
    }

    fn get_transaction_by_version(
        &self,
        args: GetTransactionByVersionArgs,
    ) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_transaction_by_version(args).await;
            return Ok(ret);
        })
    }

    fn get_accounts_transactions(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_accounts_transactions(args).await;
            return Ok(ret);
        })
    }

    fn simulate_transaction(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let data = hex::decode(args.data).unwrap();
            let accept = if args.is_bcs_format.unwrap_or(false) {
                AcceptType::Bcs
            } else {
                AcceptType::Json
            };
            let ret = vm.simulate_transaction(data, accept).await;
            Ok(ret)
        })
    }

    fn encode_submission(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.encode_submission(args.data.as_str()).await;
            return Ok(ret);
        })
    }

    fn estimate_gas_price(&self) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.estimate_gas_price().await;
            Ok(ret)
        })
    }

    fn faucet_apt(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let s = args.data.as_str();
            let acc = HexParser::parse_hex_string(s).unwrap();
            let accept = if args.is_bcs_format.unwrap_or(false) {
                AcceptType::Bcs
            } else {
                AcceptType::Json
            };
            let ret = vm.faucet_apt(acc, accept).await;
            Ok(ret)
        })
    }

    fn create_account(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let accept = if args.is_bcs_format.unwrap_or(false) {
                AcceptType::Bcs
            } else {
                AcceptType::Json
            };
            let s = args.data.as_str();
            let acc = HexParser::parse_hex_string(s).unwrap();
            let ret = vm.create_account(acc, accept).await;
            Ok(ret)
        })
    }

    fn get_account(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account(args).await;
            return Ok(ret);
        })
    }

    fn get_account_resources(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_resources(args).await;
            return Ok(ret);
        })
    }

    fn get_account_modules(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_modules(args).await;
            return Ok(ret);
        })
    }

    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_resources_state(args).await;
            return Ok(ret);
        })
    }

    fn get_account_modules_state(&self, args: AccountStateArgs) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_modules_state(args).await;
            return Ok(ret);
        })
    }

    fn get_block_by_height(&self, args: BlockArgs) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_block_by_height(args).await;
            return Ok(ret);
        })
    }

    fn get_block_by_version(&self, args: BlockArgs) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_block_by_version(args).await;
            return Ok(ret);
        })
    }

    fn view_function(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.view_function(args).await;
            return Ok(ret);
        })
    }

    fn get_table_item(&self, args: RpcTableReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_table_item(args).await;
            return Ok(ret);
        })
    }

    fn get_raw_table_item(&self, args: RpcTableReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_raw_table_item(args).await;
            return Ok(ret);
        })
    }

    fn get_events_by_creation_number(&self, args: RpcEventNumReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_events_by_creation_number(args).await;
            return Ok(ret);
        })
    }

    fn get_events_by_event_handle(&self, args: RpcEventHandleReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_events_by_event_handle(args).await;
            return Ok(ret);
        })
    }

    fn get_ledger_info(&self) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_ledger_info().await;
            return Ok(ret);
        })
    }

    fn faucet_with_cli(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let s = args.data.as_str();
            let acc = HexParser::parse_hex_string(s).unwrap();
            let ret = vm.faucet_with_cli(acc).await;
            Ok(ret)
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChainHandler<T> {
    pub handler: IoHandler,
    _marker: PhantomData<T>,
}

#[tonic::async_trait]
impl<T> Handle for ChainHandler<T>
where
    T: Rpc + Send + Sync + Clone + 'static,
{
    async fn request(
        &self,
        req: &Bytes,
        _headers: &[Element],
    ) -> io::Result<(Bytes, Vec<Element>)> {
        match self.handler.handle_request(&de_request(req)?).await {
            Some(resp) => Ok((Bytes::from(resp), Vec::new())),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to handle request",
            )),
        }
    }
}

impl<T: Rpc> ChainHandler<T> {
    pub fn new(service: T) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(Rpc::to_delegate(service));
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}

#[allow(dead_code)]
fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
