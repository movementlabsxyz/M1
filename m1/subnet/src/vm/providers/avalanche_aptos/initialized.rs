// aptos execution
use crate::executor::{
    executor::Executor,
    providers::aptos::{
        self,
        aptos::Aptos
    }
};
// avalanche state
use crate::state::avalanche::state::State;
// VM State marker trait
use super::avalanche_aptos::AvalancheAptosState;
// Block
use crate::util::types::block::Block;
use crate::state::avalanche::avalanche_block::AvalancheBlock;
use aptos_api::{Context, get_raw_api_service, RawApi};

#[derive(Debug, Clone)]
pub struct Initialized {
    pub executor : Executor<Aptos<aptos::initialized::Initialized>>,
    pub state: State,
}

impl AvalancheAptosState for Initialized {}

impl Initialized {

    pub fn new(
        executor : Executor<Aptos<Initialized>>, 
        state : State,
    ) -> Self {
        Initialized {
            executor,
            state,
        }
    }

}