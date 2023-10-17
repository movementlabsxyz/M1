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

    pub async fn parse_block(
        &self,
        bytes : &[u8]
    )->Result<AvalancheBlock, anyhow::Error> {

        let state = self.state.clone();
        let mut new_block = AvalancheBlock::from_slice(bytes, state.clone())?;
        new_block.block.set_status(choices::status::Status::Processing);
        
        match state.get_block(&new_block.id()).await {
            Ok(prev) => {
                Ok(AvalancheBlock::new(
                    prev,
                    state.clone()
                ))
            }
            Err(_) => {
                Ok(new_block)
            }
        }

    }

}

// Bubble that up to the generic
impl <S : Initialized> AvalancheAptos<S> {

    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> Result<AvalancheBlock, anyhow::Error> {

        let block = self.parse_block(bytes).await?;
        Ok(block)

    }

}


#[async_trait]
impl Parser for AvalancheAptosVm {
    type Block = AvalancheBlock;

    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> io::Result<<Self as Parser>::Block> {
        
        let block = self.parse_block(bytes).await?;
        Ok(block)

    }

}