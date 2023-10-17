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

// Implement on the initialized state
impl Initialized {

    pub async fn get_block(
        &self,
        block_id: ids::Id,
    ) -> Result<AvalancheBlock, anyhow::Error> {

        let state = self.state.clone();
        let mut block = state.get_block(&blk_id).await?;
        let mut new_state = state.clone();

        // just add Avalanche state to the block
        Ok(AvalancheBlock::new(
            block,
            new_state
        ))

    }

}

// Bubble that up to the generic
impl <S : Initialized> AvalancheAptos<S> {

    async fn get_block(
        &self,
        block_id: ids::Id,
    ) -> Result<AvalancheBlock, anyhow::Error> {

        let block = self.get_block(block_id).await?;
        Ok(block)

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

        match self.get_runtime() {
            AvalancheAptosRuntime::Initialized(initialized) => {
                initialized.get_block(blk_id).await
            },
            _ => Err(anyhow::anyhow!("AvalancheAptosVm is not initialized")),
        }
        
    }
}
