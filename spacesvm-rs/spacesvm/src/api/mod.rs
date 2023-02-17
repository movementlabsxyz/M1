pub mod client;
pub mod service;

use avalanche_types::ids;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};

use crate::chain::{
    storage::ValueMeta,
    tx::decoder::TypedData,
    tx::{self},
};

#[rpc]
pub trait Service {
    #[rpc(name = "ping", alias("spacesvm.ping"))]
    fn ping(&self) -> BoxFuture<Result<PingResponse>>;

    #[rpc(name = "issueTx", alias("spacesvm.issueTx"))]
    fn issue_tx(&self, params: IssueTxArgs) -> BoxFuture<Result<IssueTxResponse>>;

    #[rpc(name = "decodeTx", alias("spacesvm.decodeTx"))]
    fn decode_tx(&self, params: DecodeTxArgs) -> BoxFuture<Result<DecodeTxResponse>>;

    #[rpc(name = "resolve", alias("spacesvm.resolve"))]
    fn resolve(&self, params: ResolveArgs) -> BoxFuture<Result<ResolveResponse>>;
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PingResponse {
    pub success: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueRawTxArgs {
    pub tx: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueRawTxResponse {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub tx_id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueTxArgs {
    pub typed_data: TypedData,
    pub signature: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IssueTxResponse {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub tx_id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DecodeTxArgs {
    pub tx_data: tx::unsigned::TransactionData,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DecodeTxResponse {
    pub typed_data: TypedData,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResolveArgs {
    pub space: Vec<u8>,
    pub key: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ResolveResponse {
    pub exists: bool,
    pub value: Vec<u8>,
    pub meta: ValueMeta,
}

pub fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
