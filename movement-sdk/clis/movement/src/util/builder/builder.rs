use serde::{Serialize, Deserialize};
use crate::util::release::Release;
use crate::util::location::Location;
use crate::util::util::Version;
use crate::util::artifact::{Artifact, self};

#[async_trait::async_trait]
pub trait BuilderOperations {

    async fn build(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error>;

    async fn remove(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error>;

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Builder {
    KnownScript, 
    RustBuild,
    FromArchive,
    ReleaseOnly,
    Pipeline,
    Noop,
    Unknown
}

#[async_trait::async_trait]
impl BuilderOperations for Builder {

    async fn build(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        todo!();

    }

    async fn remove(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        todo!();

    }

}
