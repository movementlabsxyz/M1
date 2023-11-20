use avalanche_types::{
    choices,
    ids::{self, Id},
    subnet::rpc::consensus::snowman::{
        Decidable,
        Block
    }
};
use movement_sdk::DataAvailabilityLayer;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::io;

#[derive(Debug, Clone)]
pub struct AvalancheBlock<
    Block, 
    BlockId,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId>
>{
    pub inner_block: Block,
    pub data_availability_layer: Arc<RwLock<DA>>,
}

impl <
    Block, 
    BlockId,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId>
> AvalancheBlock<Block, BlockId, DA> {

    pub fn new(
        inner_block: Block,
        data_availability_layer: Arc<RwLock<DA>>,
    ) -> Self {
        Self {
            inner_block,
            data_availability_layer,
        }
    }

}

impl <
    Block, 
    BlockId,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId>
> Into<AvalancheBlock<Block, BlockId, DA>> for (Block, Arc<RwLock<DA>>) {
    fn into(self) -> AvalancheBlock<Block, BlockId, DA> {
        AvalancheBlock::new(self.0, self.1)
    }
}

#[async_trait::async_trait]
impl <
    Block : Sync + Clone, 
    BlockId,
    DA : DataAvailabilityLayer<Block = Block, BlockId = BlockId> + Sync + Send
> Decidable for AvalancheBlock<Block, BlockId, DA> {
    
    async fn id(&self) -> ids::Id {
        unimplemented!("todo: implement id");
        // ids::Id::empty()
    }

    /// Implements "snowman.Block.choices.Decidable"
    async fn status(&self) -> choices::status::Status {
        unimplemented!("todo: implement status");
        // choices::status::Status::Unknown(String::from("unimplemented"))
    }

    async fn accept(&mut self) -> io::Result<()> {
        let data_availability_layer = self.data_availability_layer.read().await;
        data_availability_layer.accept_block(self.inner_block.clone()).await?;
        Ok(())
    }

    async fn reject(&mut self) -> io::Result<()> {
        let data_availability_layer = self.data_availability_layer.read().await;
        data_availability_layer.reject_block(self.inner_block.clone()).await?;
        Ok(())
    }

}