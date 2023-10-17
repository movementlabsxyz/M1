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
use avalanche_types::subnet::rpc::snow::engine::common::engine::{AppHandler, CrossChainAppHandler, NetworkAppHandler};

#[async_trait]
impl NetworkAppHandler for AvalancheAptosVm {
    /// Currently, no app-specific messages, so returning Ok.
    async fn app_request(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _deadline: DateTime<Utc>,
        _request: &[u8],
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no app-specific messages, so returning Ok.
    async fn app_request_failed(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
    ) -> io::Result<()> {
        Ok(())
    }

    /// Currently, no app-specific messages, so returning Ok.
    async fn app_response(
        &self,
        _node_id: &ids::node::Id,
        _request_id: u32,
        _response: &[u8],
    ) -> io::Result<()> {
        Ok(())
    }

    async fn app_gossip(&self, _node_id: &ids::node::Id, msg: &[u8]) -> io::Result<()> {
        match serde_json::from_slice::<SignedTransaction>(msg) {
            Ok(s) => {
                self.add_pool(s).await;
            }
            Err(_) => {}
        }
        Ok(())
    }
}