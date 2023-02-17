//! Implements chain/VM specific handlers.
//! To be served via `[HOST]/ext/bc/[CHAIN ID]/rpc`.

use std::str::FromStr;

use crate::{block::Block, vm};
use avalanche_types::ids;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

/// Defines RPCs specific to the chain.
#[rpc]
pub trait Rpc {
    /// Pings the VM.
    #[rpc(name = "ping", alias("timestampvm.ping"))]
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>>;

    /// Proposes the arbitrary data.
    #[rpc(name = "proposeBlock", alias("timestampvm.proposeBlock"))]
    fn propose_block(&self, args: ProposeBlockArgs) -> BoxFuture<Result<ProposeBlockResponse>>;

    /// Fetches the last accepted block.
    #[rpc(name = "lastAccepted", alias("timestampvm.lastAccepted"))]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    /// Fetches the block.
    #[rpc(name = "getBlock", alias("timestampvm.getBlock"))]
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProposeBlockArgs {
    #[serde(with = "avalanche_types::codec::serde::base64_bytes")]
    pub data: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProposeBlockResponse {
    /// TODO: returns Id for later query, using hash + time?
    pub success: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LastAcceptedResponse {
    pub id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockArgs {
    /// TODO: use "ids::Id"
    /// if we use "ids::Id", it fails with:
    /// "Invalid params: invalid type: string \"g25v3qDyAaHfR7kBev8tLUHouSgN5BJuZjy1BYS1oiHd2vres\", expected a borrowed string."
    pub id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockResponse {
    pub block: Block,
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
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>> {
        log::debug!("ping called");
        Box::pin(async move { Ok(crate::api::PingResponse { success: true }) })
    }

    fn propose_block(&self, args: ProposeBlockArgs) -> BoxFuture<Result<ProposeBlockResponse>> {
        log::debug!("propose_block called");
        let vm = self.vm.clone();

        Box::pin(async move {
            vm.propose_block(args.data)
                .await
                .map_err(create_jsonrpc_error)?;
            Ok(ProposeBlockResponse { success: true })
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

                return Ok(GetBlockResponse { block });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }
}

fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
