use crate::block::{Block, VerifiedExecutableBlock};

#[async_trait::async_trait]
pub trait VerifiedExecutableBlockProvider {

    /// Provides a verified executable block.
    async fn verified_executable_block(&self, block : &Block) -> Result<VerifiedExecutableBlock, anyhow::Error>;

}