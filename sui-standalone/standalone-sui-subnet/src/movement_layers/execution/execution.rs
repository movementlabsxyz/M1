use movement_sdk::ExecutionLayer;
use async_channel::{Sender, Receiver};
use crate::util::types::block::Block;
use tonic::async_trait;


#[derive(Debug, Clone)]
pub struct SuiChannelExecutionLayer {
    pub block_sender: Sender<Block>,
    pub block_receiver: Receiver<Block>,
}

#[async_trait]
impl ExecutionLayer for SuiChannelExecutionLayer {
    type Block = Block;
    type BlockId = String;
    type ChangeSet = String;

    // Gets the next block from the previous layer.
    async fn get_next_block(
        &self
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        Ok(Some(self.block_receiver.recv().await?))
    }

    // Executes a block and produces a change set.
    async fn execute_block(
        &self,
        block: Self::Block
    ) -> Result<Self::ChangeSet, anyhow::Error> {
        unimplemented!()
    }

    // Sends a change set to the next layer,  i.e., the storage layer.
    async fn send_change_set(
        &self,
        change_set: Self::ChangeSet
    ) -> Result<(), anyhow::Error> {
        unimplemented!()
    }

    // Gets an executed block
    async fn get_block(
        &self,
        block_id: Self::BlockId
    ) -> Result<Option<Self::Block>, anyhow::Error> {
        unimplemented!()
    }

}