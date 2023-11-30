use async_trait::async_trait;
use super::super::Service;
use super::{
    proxy::M1SubnetProxyService,
    subnet::M1SubnetService,
};


#[derive(Debug)]
pub struct M1Service {
    pub subnet: M1SubnetService,
    pub proxy: M1SubnetProxyService,
}

#[async_trait]
impl Service for M1Service {

    async fn get_name(&self) -> String {
        "m1".to_string()
    }

    async fn start(&self) -> Result<(), anyhow::Error> {

        tokio::try_join!(
            self.subnet.start(),
            self.proxy.start(),
        )?;

        Ok(())

    }

    async fn stop(&self) -> Result<(), anyhow::Error> {
        
        tokio::try_join!(
            self.subnet.stop(),
            self.proxy.stop(),
        )?;

        Ok(())

    }

    async fn status(&self) -> Result<(), anyhow::Error> {
        
        tokio::try_join!(
            self.subnet.status(),
            self.proxy.status(),
        )?;

        Ok(())
        
    }

}