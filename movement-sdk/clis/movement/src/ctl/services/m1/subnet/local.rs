use async_trait::async_trait;
use super::super::super::Service;

#[derive(Debug)]
pub struct M1SubnetLocalService {

}

#[async_trait]
impl Service for M1SubnetLocalService {

    async fn get_name(&self) -> String {
        "m1-subnet-local".to_string()
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