pub mod base;
pub mod claim;
pub mod decoder;
pub mod delete;
pub mod set;
pub mod tx;
pub mod unsigned;

use std::io::Result;

use avalanche_types::{ids, subnet};

use crate::block::Block;

#[tonic::async_trait]
#[typetag::serde(tag = "type")]
pub trait Transaction {
    async fn init(&mut self) -> Result<()>;
    async fn bytes(&self) -> &Vec<u8>;
    async fn size(&self) -> u64;
    async fn id(&self) -> ids::Id;
    async fn execute(
        &self,
        db: &'life1 Box<dyn subnet::rpc::database::Database + Send + Sync>,
        block: &Block,
    ) -> Result<()>;
}
