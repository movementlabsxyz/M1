use serde::{Serialize, Deserialize};
use crate::util::release::Release;
use crate::util::location::Location;
use crate::util::util::Version;

#[async_trait::async_trait]
pub trait BuilderOperations {

    async fn build(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error>;

    async fn remove(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error>;

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Builder {
    KnownScript, 
    RustBuild,
    FromArchive,
    ReleaseOnly,
    Pipeline,
    Unknown
}

#[async_trait::async_trait]
impl BuilderOperations for Builder {

    async fn build(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error> {
        todo!()
    }

    async fn remove(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error> {
        todo!()
    }

}
