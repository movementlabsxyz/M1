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

// Bubble that up to the generic
impl AvalancheAptos<Initialized> {

    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> Result<AvalancheBlock, anyhow::Error> {

        let state = self.state.state.clone();
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


#[async_trait]
impl Parser for AvalancheAptosVm {
    type Block = AvalancheBlock;

    async fn parse_block(
        &self,
        bytes: &[u8],
    ) -> io::Result<<Self as Parser>::Block> {
        
        match self.get_runtime().await? {
            AvalancheAptosRuntime::Initialized(initialized) => {
                initialized.parse_block(bytes).await
            }
            _ => {
                Err(io::Error::new(io::ErrorKind::Other, "Uninitialized"))
            }
        }

    }

}