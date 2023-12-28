use serde::{Serialize, Deserialize};
use crate::util::artifact::{Artifact, ArtifactStatus};

#[async_trait::async_trait]
pub trait CheckerOperations {

    async fn check(&self, artifact : &Artifact) -> Result<ArtifactStatus, anyhow::Error>;

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Checker {
    Noop,
    Unknown
}

#[async_trait::async_trait]
impl CheckerOperations for Checker {

    async fn check(&self, artifact : &Artifact) -> Result<ArtifactStatus, anyhow::Error> {
        todo!()
    }

}