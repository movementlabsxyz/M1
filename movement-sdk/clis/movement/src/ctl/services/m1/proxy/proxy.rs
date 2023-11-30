use async_trait::async_trait;
use clap::Parser;
use super::super::super::Service;

#[derive(Parser, Debug)]
pub struct M1SubnetProxyService {

}

#[async_trait]
impl Service for M1SubnetProxyService {

    async fn get_name(&self) -> String {
        "m1-subnet-proxy".to_string()
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