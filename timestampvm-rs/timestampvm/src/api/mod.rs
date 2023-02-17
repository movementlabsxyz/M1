//! Implementation of timestampvm APIs, to be registered via
//! `create_static_handlers` and `create_handlers` in the [`vm`](crate::vm) crate.

pub mod chain_handlers;
pub mod static_handlers;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PingResponse {
    pub success: bool,
}
