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

#[derive(Debug, Clone)]
pub struct Initialized {
    pub executor : Executor<Aptos<aptos::initialized::Initialized>>,
    pub state: State,
}

impl AvalancheAptosState for Initialized {}

impl Initialized {

    pub fn new(executor : Executor<Aptos<Initialized>>, state : State) -> Self {
        Initialized {
            executor,
            state,
        }
    }

}

// Block building
impl Initialized {

    // reads in transactions and then builds a block
    pub async fn propose_block(&self) -> Result<AvalancheBlock, anyhow::Error> {

        let state = self.state;
        let executor = self.executor;
        let parent_block = state.get_block(&state.preferred).await?;
        let parent_block_id = parent_block.id();
        let height = parent_block.height() + 1;

        let block = executor.propose_block(parent_block_id, height).await?;
        let avalanche_block = AvalancheBlock::new(
            block,
            state.clone()
        );

        Ok(avalanche_block)
     
    }

}