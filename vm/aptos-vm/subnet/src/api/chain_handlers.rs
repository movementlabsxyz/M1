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

    /// get the account sequence number for submit transaction
    #[rpc(name = "getSequenceNumber", alias("aptosvm.getSequenceNumber"))]
    fn get_sequence_number(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>>;

    /// get the account sequence number for submit transaction
    #[rpc(name = "getBalance", alias("aptosvm.getBalance"))]
    fn get_balance(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>>;
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
pub struct GetBlockResponse {
    pub block: Block,
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountArgs {
    pub account: String,
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


    fn facet_apt(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>> {
        log::debug!("facet_apt called");
        let mut vm = self.vm.clone();
        Box::pin(async move {
            let acc = hex::decode(args.account).unwrap();
            let hash = vm.facet_apt(acc).await;
            Ok(AccountStrRes { data: hex::encode(hash) })
        })
    }

    fn create_account(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>> {
        log::debug!("create_account called");
        let mut vm = self.vm.clone();
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


    fn get_sequence_number(&self, args: AccountArgs) -> BoxFuture<Result<AccountNumberRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let reader = vm.state.read().await;
            let db = reader.db.as_ref().unwrap();
            let acc = hex::decode(args.account).unwrap();
            let acc_source = vm.get_account_resource(&db, &acc);
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
}


fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
