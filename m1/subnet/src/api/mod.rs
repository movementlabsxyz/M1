//! Implementation of timestampvm APIs, to be registered via
//! `create_static_handlers` and `create_handlers` in the [`vm`](crate::vm) crate.

use std::io;

use bytes::Bytes;
use jsonrpc_core::MethodCall;
use serde::{Deserialize, Serialize};
use avalanche_types::proto::http::Element;
use avalanche_types::subnet::rpc::http::handle::Handle;

pub mod chain_handlers;
pub mod static_handlers;
pub mod eth_rpc_handlers;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PingResponse {
    pub success: bool,
}
pub fn de_request(req: &Bytes) -> io::Result<String> {
    let method_call: MethodCall = serde_json::from_slice(req).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to deserialize request: {e}"),
        )
    })?;
    serde_json::to_string(&method_call).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to serialize request: {e}"),
        )
    })
}

#[derive(Clone)]
pub enum ChainHandler {
    ChainHandler(chain_handlers::ChainHandler<chain_handlers::ChainService>),
    EthHandler(eth_rpc_handlers::EthHandler<eth_rpc_handlers::EthService>),
}

#[tonic::async_trait]
impl Handle for ChainHandler {
    async fn request(
        &self,
        req: &Bytes,
        _headers: &[Element],
    ) -> io::Result<(Bytes, Vec<Element>)> {
        match self {
            ChainHandler::ChainHandler(handler) => handler.request(req, _headers).await,
            ChainHandler::EthHandler(handler) => handler.request(req, _headers).await,
        }
    }
}