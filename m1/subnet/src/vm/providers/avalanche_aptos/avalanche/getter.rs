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
use avalanche_types::subnet::rpc::snowman::block::{BatchedChainVm, ChainVm, Getter, Parser};

impl AvalancheAptos<Initialized> {

    async fn get_block(
        &self,
        block_id: ids::Id,
    ) -> Result<AvalancheBlock, anyhow::Error> {

        let state = self.state.state.clone();
        let mut block = state.get_block(&blk_id).await?;
        let mut new_state = state.clone();

        // just add Avalanche state to the block
        Ok(AvalancheBlock::new(
            block,
            new_state
        ))

    }

}

// Implement the getter
#[async_trait]
impl Getter for AvalancheAptosVm {
    type Block = AvalancheBlock;

    async fn get_block(
        &self,
        block_id: ids::Id,
    ) -> io::Result<<Self as Getter>::Block> {

        match self.get_runtime().await? {
            AvalancheAptosRuntime::Initialized(initialized) => {
                initialized.get_block(block_id).await
            },
            _ => Err(anyhow::anyhow!("AvalancheAptosVm is not initialized")),
        }
        
    }
}
