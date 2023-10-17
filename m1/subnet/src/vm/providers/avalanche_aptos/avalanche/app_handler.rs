use tonic::async_trait;
use crate::state::avalanche::avalanche_block::AvalancheBlock;
use super::super::{
    avalanche_aptos::{
        AvalancheAptos,
        AvalancheAptosVm,
        AvalancheAptosRuntime
    },
    initialized::Initialized,
};
use avalanche_types::subnet::rpc::snow::engine::common::engine::{AppHandler, CrossChainAppHandler, NetworkAppHandler};

impl AppHandler for AvalancheAptosVm {}