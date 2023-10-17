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

// Bubble up to the generic
impl AvalancheAptos<Initialized> {

    async fn build_block(&self) -> Result<(), anyhow::Error> {

        let state = self.state.state;
        let executor = self.state.executor;
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



#[async_trait]
impl ChainVm for AvalancheAptosVm {

    // must be AvalancheBlock because AvalancheBlock must be Decidable and implement Block trait
    type Block = AvalancheBlock;

    async fn build_block(
        &self,
    ) -> io::Result<<Self as ChainVm>::Block> {

        match self.get_runtime().await? {
            AvalancheAptosRuntime::Initialized(initialized) => {
                initialized.build_block().await
            }
            _ => {
                Err(io::Error::new(io::ErrorKind::Other, "Uninitialized"))
            }
        }

    }


}