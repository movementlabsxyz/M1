use async_trait::async_trait;
use super::super::super::Service;
use super::{
    M1SubnetFujiService,
    M1SubnetLocalService
};

#[derive(Debug)]
pub enum M1SubnetService {
    Local(M1SubnetLocalService),
    Fuji(M1SubnetFujiService),
}

#[async_trait]
impl Service for M1SubnetService {

    async fn get_name(&self) -> String {
        // todo: this may need to be the name from the inner service
        "m1-subnet".to_string()
    }

    async fn start(&self) -> Result<(), anyhow::Error> {
        match self {
            M1SubnetService::Local(service) => service.start().await,
            M1SubnetService::Fuji(service) => service.start().await,
        }
    }

    async fn stop(&self) -> Result<(), anyhow::Error> {
        match self {
            M1SubnetService::Local(service) => service.stop().await,
            M1SubnetService::Fuji(service) => service.stop().await,
        }
    }

    async fn status(&self) -> Result<(), anyhow::Error> {
        match self {
            M1SubnetService::Local(service) => service.status().await,
            M1SubnetService::Fuji(service) => service.status().await,
        }
    }

}

impl Into<M1SubnetService> for M1SubnetLocalService {
    fn into(self) -> M1SubnetService {
        M1SubnetService::Local(self)
    }
}

impl Into<M1SubnetService> for M1SubnetFujiService {
    fn into(self) -> M1SubnetService {
        M1SubnetService::Fuji(self)
    }
}