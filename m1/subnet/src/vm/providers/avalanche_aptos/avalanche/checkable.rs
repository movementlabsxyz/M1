use tonic::async_trait;
use super::super::avalanche_aptos::AvalancheAptosVm;
use avalanche_types::subnet::rpc::health::Checkable;

#[async_trait]
impl Checkable for AvalancheAptosVm {
    async fn health_check(&self) -> io::Result<Vec<u8>> {
        Ok("200".as_bytes().to_vec())
    }
}