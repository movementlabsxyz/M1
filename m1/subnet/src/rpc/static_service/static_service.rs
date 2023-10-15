use jsonrpc_core::{BoxFuture, Result};
use serde::{Deserialize, Serialize};
use aptos_api_types::U64;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PingResponse {
    pub success: bool,
}

#[rpc]
pub trait StaticService {
    #[rpc(name = "ping", alias("timestampvm.ping"))]
    fn ping(&self) -> BoxFuture<Result<PingResponse>>;
}