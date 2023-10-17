use jsonrpc_core::{BoxFuture, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use aptos_api_types::U64;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTableItemArgs {
    pub table_handle: String,
    pub key_type: String,
    pub value_type: String,
    pub key: String,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcReq {
    pub data: String,
    pub ledger_version: Option<U64>,
    pub start: Option<String>,
    pub limit: Option<u16>,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcRes {
    pub data: String,
    pub header: String,
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcTableReq {
    pub query: String,
    pub body: String,
    pub ledger_version: Option<U64>,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcEventNumReq {
    pub address: String,
    pub creation_number: U64,
    pub start: Option<U64>,
    pub limit: Option<u16>,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RpcEventHandleReq {
    pub start: Option<U64>,
    pub limit: Option<u16>,
    pub address: String,
    pub event_handle: String,
    pub field_name: String,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct BlockArgs {
    pub height_or_version: u64,
    pub with_transactions: Option<bool>,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTransactionByVersionArgs {
    pub version: U64,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountStateArgs {
    pub account: String,
    pub resource: String,
    pub ledger_version: Option<U64>,
    pub is_bsc_format: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PageArgs {
    pub start: Option<U64>,
    pub limit: Option<u16>,
    pub is_bsc_format: Option<bool>,
}

#[rpc]
pub trait ChainServiceRpc {
    /*******************************TRANSACTION START***************************************/
    #[rpc(name = "getTransactions", alias("aptosvm.getTransactions"))]
    fn get_transactions(&self, args: PageArgs) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "submitTransaction", alias("aptosvm.submitTransaction"))]
    fn submit_transaction(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "submitTransactionBatch", alias("aptosvm.submitTransactionBatch"))]
    fn submit_transaction_batch(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getTransactionByHash", alias("aptosvm.getTransactionByHash"))]
    fn get_transaction_by_hash(&self, args: RpcReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getTransactionByVersion", alias("aptosvm.getTransactionByVersion"))]
    fn get_transaction_by_version(&self, args: GetTransactionByVersionArgs) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getAccountsTransactions", alias("aptosvm.getAccountsTransactions"))]
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

    #[rpc(name = "getAccountResourcesState", alias("aptosvm.getAccountResourcesState"))]
    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getAccountModulesState", alias("aptosvm.getAccountModulesState"))]
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

    #[rpc(name = "getEventsByCreationNumber", alias("aptosvm.getEventsByCreationNumber"))]
    fn get_events_by_creation_number(&self, args: RpcEventNumReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getEventsByEventHandle", alias("aptosvm.getEventsByEventHandle"))]
    fn get_events_by_event_handle(&self, args: RpcEventHandleReq) -> BoxFuture<Result<RpcRes>>;

    #[rpc(name = "getLedgerInfo", alias("aptosvm.getLedgerInfo"))]
    fn get_ledger_info(&self) -> BoxFuture<Result<RpcRes>>;
}