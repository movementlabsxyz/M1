use crate::util::types::block::Block;
use tonic::async_trait;
use avalanche_types::subnet::rpc::consensus::snowman::{
    Decidable,
    Block as AvalancheBlockOperations
};
use super::state::State;

pub struct AvalancheBlock {
    pub block : Block,
    pub state : State,
}

impl AvalancheBlock {

    pub fn new(block : Block, state : State) -> Self {
        AvalancheBlock {
            block,
            state,
        }
    }

    /// Loads [`Block`](Block) from JSON bytes.
    pub fn from_slice(bytes: impl AsRef<[u8]>, state : State) -> Result<Self, anyhow::Error> {
        let block = Block::from_slice(bytes)?;
        Ok(Self::new(
            block,
            state
        ))
    }

}

#[async_trait]
impl Decidable for AvalancheBlock {

    async fn id(&self) -> io::Result<ids::Id> {
        Ok(self.block.id())
    }
    
    async fn verify(&self) -> io::Result<()> {
        self.state.verify_block(&self.block).await?;
        Ok(())
    }

    async fn accept(&mut self) -> io::Result<()> {
        self.state.accept_block(&self.block).await?;
        Ok(())
    }

    async fn reject(&mut self) -> io::Result<()> {
        self.state.reject_block(&self.block).await?;
        Ok(())
    }

}

#[async_trait]
impl AvalancheBlockOperations for AvalancheBlock {

    async fn bytes(&self) -> &[u8] {
        self.block.bytes().await
    }

    async fn height(&self) -> u64 {
        self.block.height().await
    }

    async fn timestamp(&self) -> u64 {
        self.block.timestamp().await
    }

    async fn parent(&self) -> ids::Id {
        self.block.parent_id().await
    }

    async fn status(&self) -> choices::status::Status {
        self.block.status().await
    }

}
