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

#[async_trait]
impl BatchedChainVm for AvalancheAptosVm {
    type Block = AvalancheBlock;

    async fn get_ancestors(
        &self,
        _block_id: ids::Id,
        _max_block_num: i32,
        _max_block_size: i32,
        _max_block_retrival_time: Duration,
    ) -> io::Result<Vec<Bytes>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "get_ancestors not implemented",
        ))
    }

    async fn batched_parse_block(&self, _blocks: &[Vec<u8>]) -> io::Result<Vec<Self::Block>> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "batched_parse_block not implemented",
        ))
    }

}
