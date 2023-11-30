use async_trait::async_trait;

#[async_trait]
pub trait Service {
    
    async fn get_name(&self) -> String;
    
    async fn start(&self) -> Result<(), anyhow::Error>;
    
    async fn stop(&self) -> Result<(), anyhow::Error>;
    
    async fn status(&self) -> Result<(), anyhow::Error>;
    
}