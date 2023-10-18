use aptos_api::transactions::TransactionsApi;
use tonic::async_trait;
use crate::util::types::block::Block;
use avalanche_types::{choices, ids};

#[async_trait]
pub trait Uninitialized {
    type Initialized;
    type Config;

    async fn initialize(self, config : Self::Config) -> Result<Self::Initialized, anyhow::Error>;
}

#[async_trait]
pub trait Initialized {
    type ExecutionResult;
    type Api;

    async fn propose_block(
        &self, 
        parent_block_id : ids::Id,
        height : u64,
    ) -> Result<Block, anyhow::Error>;

    async fn execute_block(&self, block : Block) -> Result<Self::ExecutionResult, anyhow::Error>;

    async fn commit_block(&self, block : Self::ExecutionResult) -> Result<(), anyhow::Error>;

    async fn apply_block(&self, block : Block) -> Result<(), anyhow::Error> {
        let result = self.execute_block(block).await?;
        self.commit_block(result).await?;
        Ok(())
    }

    async fn get_api(&self) -> Result<Self::Api, anyhow::Error>;

    async fn get_transactions_api(&self) -> Result<TransactionsApi, anyhow::Error>;

}

pub struct Executor<S> {
    state : S
}

impl <S> Executor<S> {
    pub fn new(state : S) -> Self {
        Executor {
            state,
        }
    }
}

impl <S : Uninitialized> Executor<S> {

    pub async fn initialize(self, config : S::Config) -> Result<Executor<S::Initialized>, anyhow::Error> {
        Ok(Executor::new(
            self.state.initialize(config).await?
        ))
    }

}

impl <S : Initialized> Executor<S> {
    
    pub async fn propose_block(
        &self, 
        parent_block_id : ids::Id,
        height : u64,
    ) -> Result<Block, anyhow::Error> {
        self.state.propose_block(parent_block_id, height).await
    }

    pub async fn execute_block(&self, block : Block) -> Result<S::ExecutionResult, anyhow::Error> {
        self.state.execute_block(block).await
    }

    pub async fn commit_block(&self, block : S::ExecutionResult) -> Result<(), anyhow::Error> {
        self.state.commit_block(block).await
    }

    pub async fn apply_block(&self, block : Block) -> Result<(), anyhow::Error> {
        self.state.apply_block(block).await
    }
    
    pub async fn get_api(&self) -> Result<S::Api, anyhow::Error> {
        self.state.get_api().await
    }

    pub async fn get_transactions_api(&self) -> Result<TransactionsApi, anyhow::Error> {
        self.state.get_transactions_api().await
    }

}