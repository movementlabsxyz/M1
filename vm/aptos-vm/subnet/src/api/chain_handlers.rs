//! Implements chain/VM specific handlers.
//! To be served via `[HOST]/ext/bc/[CHAIN ID]/rpc`.

use std::io;
use std::str::FromStr;

use avalanche_types::subnet::rpc::snowman::block::Getter;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use avalanche_types::{choices, codec::serde::hex_0x_bytes::Hex0xBytes, ids, subnet};
use aptos_sdk::rest_client::aptos_api_types::MAX_RECURSIVE_TYPES_ALLOWED;
use aptos_sdk::rest_client::aptos_api_types::mime_types::BCS;
use aptos_types::transaction::{SignedTransaction, Transaction, TransactionOutput, TransactionPayload};
use serde_with::serde_as;
use aptos_api::response::BasicResponse;
use crate::{block::Block, vm};

/// Defines RPCs specific to the chain.
#[rpc]
pub trait Rpc {
    /// Proposes the arbitrary data.
    #[rpc(name = "submitTransaction", alias("aptosvm.submitTransaction"))]
    fn submit_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>>;

    #[rpc(name = "simulateTransaction", alias("aptosvm.simulateTransaction"))]
    fn simulate_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>>;

    #[rpc(name = "estimateGasPrice", alias("aptosvm.estimateGasPrice"))]
    fn estimate_gas_price(&self) -> BoxFuture<Result<SubmitTransactionRes>>;

    /// faucet token
    #[rpc(name = "faucet", alias("aptosvm.faucet"))]
    fn facet_apt(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>>;
    /// faucet token
    ///
    #[rpc(name = "createAccount", alias("aptosvm.createAccount"))]
    fn create_account(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>>;
    /// Fetches the last accepted block.
    ///
    #[rpc(name = "lastAccepted", alias("aptosvm.lastAccepted"))]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    /// Fetches the block.
    #[rpc(name = "getBlock", alias("aptosvm.getBlock"))]
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;

    /// Fetches the block.
    #[rpc(name = "getTransactionByHash", alias("aptosvm.getTransactionByHash"))]
    fn get_transaction_by_hash(&self, args: GetBlockArgs) -> BoxFuture<Result<GetTransactionRes>>;

    #[rpc(name = "getSequenceNumber", alias("aptosvm.getSequenceNumber"))]
    fn get_sequence_number(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>>;

    #[rpc(name = "getBalance", alias("aptosvm.getBalance"))]
    fn get_balance(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>>;

    #[rpc(name = "viewFunction", alias("aptosvm.viewFunction"))]
    fn view_function(&self, args: ViewFunctionArgs) -> BoxFuture<Result<SubmitTransactionArgs>>;

    #[rpc(name = "getLedgerInfo", alias("aptosvm.getLedgerInfo"))]
    fn get_ledger_info(&self) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountResources", alias("aptosvm.getAccountResources"))]
    fn get_account_resources(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccount", alias("aptosvm.getAccount"))]
    fn get_account(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountModules", alias("aptosvm.getAccountModules"))]
    fn get_account_modules(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountResourcesState", alias("aptosvm.getAccountResourcesState"))]
    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountsTransactions", alias("aptosvm.getAccountsTransactions"))]
    fn get_accounts_transactions(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ViewFunctionArgs {
    // pub function: String,
    // pub type_arguments: Vec<String>,
    // pub arguments: Vec<String>,
    pub data: String,
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitTransactionArgs {
    ///  bsc payload as hex
    pub data: String,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitTransactionRes {
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LastAcceptedResponse {
    pub id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockArgs {
    pub id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTransactionRes {
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockResponse {
    pub block: Block,
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountArgs {
    pub account: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountStateArgs {
    pub account: String,
    pub resource: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountNumberRes {
    pub data: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountStrRes {
    pub data: String,
}


/// Implements API services for the chain-specific handlers.
pub struct Service {
    pub vm: vm::Vm,
}

impl Service {
    pub fn new(vm: vm::Vm) -> Self {
        Self { vm }
    }
}

impl Rpc for Service {
    fn submit_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>> {
        log::debug!("submit_transaction called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let r = vm.submit_transaction2(hex::decode(args.data).unwrap()).await;
            Ok(SubmitTransactionRes { data: r })
        })
    }

    fn simulate_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let data = hex::decode(args.data).unwrap();
            let ret = vm.simulate_transaction(data).await;
            Ok(SubmitTransactionRes { data: ret })
        })
    }

    fn estimate_gas_price(&self) -> BoxFuture<Result<SubmitTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.estimate_gas_price().await;
            Ok(SubmitTransactionRes { data: ret })
        })
    }

    fn facet_apt(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>> {
        log::debug!("facet_apt called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let acc = hex::decode(args.account).unwrap();
            let hash = vm.facet_apt(acc).await;
            Ok(AccountStrRes { data: hex::encode(hash) })
        })
    }

    fn create_account(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>> {
        log::debug!("create_account called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let hash = vm.create_account(args.account.as_str()).await;
            Ok(AccountStrRes { data: hex::encode(hash) })
        })
    }

    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>> {
        log::debug!("last accepted method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let vm_state = vm.state.read().await;
            if let Some(state) = &vm_state.state {
                let last_accepted = state
                    .get_last_accepted_block_id()
                    .await
                    .map_err(create_jsonrpc_error)?;

                return Ok(LastAcceptedResponse { id: last_accepted });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>> {
        let blk_id = ids::Id::from_str(&args.id).unwrap();
        log::info!("get_block called for {}", blk_id);

        let vm = self.vm.clone();

        Box::pin(async move {
            let vm_state = vm.state.read().await;
            if let Some(state) = &vm_state.state {
                let block = state
                    .get_block(&blk_id)
                    .await
                    .map_err(create_jsonrpc_error)?;
                let data = String::from_utf8_lossy(&block.data()).to_string();
                return Ok(GetBlockResponse { block, data });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }

    fn get_transaction_by_hash(&self, args: GetBlockArgs) -> BoxFuture<Result<GetTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let h = args.id.as_str();
            log::info!("hash by {}",h.clone());
            let ret = vm.get_transaction_by_hash(h).await;
            return Ok(GetTransactionRes { data: ret });
        })
    }

    fn get_sequence_number(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let reader = vm.state.read().await;
            let db = reader.db.as_ref().unwrap();
            let acc = hex::decode(args.account).unwrap();
            let acc_source = vm.get_account_resource_me(&db, &acc);
            let source = acc_source.unwrap();
            return Ok(AccountNumberRes { data: source.sequence_number() });
        })
    }

    fn get_balance(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let reader = vm.state.read().await;
            let db = reader.db.as_ref().unwrap();
            let acc = hex::decode(args.account).unwrap();
            let acc_source = vm.get_account_balance(&db, &acc);
            return Ok(AccountNumberRes { data: acc_source });
        })
    }

    fn view_function(&self, args: ViewFunctionArgs) -> BoxFuture<Result<SubmitTransactionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.view_function(args.data.as_str()).await;
            return Ok(SubmitTransactionArgs { data: ret });
        })
    }

    fn get_ledger_info(&self) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_ledger_info().await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account_resources(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_resources(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account_modules(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_modules(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_resources_state(args.account.as_str(),
                                                     args.resource.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_accounts_transactions(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_accounts_transactions(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }
}


fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
