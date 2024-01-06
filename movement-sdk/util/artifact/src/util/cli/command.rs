use async_trait::async_trait;
use anyhow;

#[async_trait]
pub trait Command<T> {

    async fn get_name(&self) -> String;
    
    async fn execute(self) -> Result<T, anyhow::Error>;

}