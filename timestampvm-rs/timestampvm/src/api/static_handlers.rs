//! Implements static handlers specific to this VM.
//! To be served via `[HOST]/ext/vm/[VM ID]/static`.

use crate::vm;
use jsonrpc_core::{BoxFuture, Result};
use jsonrpc_derive::rpc;

/// Defines static handler RPCs for this VM.
#[rpc]
pub trait Rpc {
    #[rpc(name = "ping", alias("timestampvm.ping"))]
    fn ping(&self) -> BoxFuture<Result<crate::api::PingResponse>>;
}

/// Implements API services for the static handlers.
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
}
