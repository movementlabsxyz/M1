use async_trait::async_trait;
use super::super::super::Service;

#[derive(Debug)]
pub struct M1SubnetFujiService {

}

#[async_trait]
impl Service for M1SubnetFujiService {

    async fn get_name(&self) -> String {
        "m1-subnet-fuji".to_string()
    }

    async fn start(&self) -> Result<(), anyhow::Error> {
        unimplemented!();
        Ok(())
    }

    async fn stop(&self) -> Result<(), anyhow::Error> {
        unimplemented!();
        Ok(())
    }

    async fn status(&self) -> Result<(), anyhow::Error> {
        unimplemented!();
        Ok(())
    }

}