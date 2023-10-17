use aptos_config::config::NodeConfig;
use tonic::async_trait;

use super::super::super::executor::{
    Initialized as InitializedExecutor, 
    Uninitialized as UninitializedExecutor
};
use super::initialized::Initialized;
use super::uninitialized::Uninitialized;

pub trait AptosState {}

// todo: potentially migrate the executor to use this wrapping
#[derive(Debug, Clone)]
pub struct Aptos<S : AptosState> {
    pub state: S,
}

impl <S> Aptos<S> where S : AptosState {
    pub fn new(state : S) -> Self {
        Aptos {
            state,
        }
    }
}

impl Default for Aptos<Uninitialized> {
    fn default() -> Self {
        Aptos {
            state: Uninitialized::default(),
        }
    }
}

#[async_trait]
impl UninitializedExecutor for Aptos<Uninitialized> {
    type Initialized = Aptos<Initialized>;
    type Config = NodeConfig;

    async fn initialize(self, config : Self::Config) -> Result<Self::Initialized, anyhow::Error> {
        Ok(Aptos::new(self.state.initialize(config).await?))
    }
}

#[async_trait]
impl InitializedExecutor for Aptos<Initialized> {

    async fn propose_block(
        &self, 
        parent_block_id : ids::Id,
        height : u64,
    ) -> Result<Block, anyhow::Error> {
        self.state.propose_block(parent_block_id, height).await
    }

    async fn execute_block(&self, block : Block) -> Result<Self::ExecutionResult, anyhow::Error> {
        self.state.execute_block(block).await
    }

    async fn commit_block(&self, block : Self::ExecutionResult) -> Result<(), anyhow::Error> {
        self.state.commit_block(block).await
    }

}
