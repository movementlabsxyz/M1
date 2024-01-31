use serde::{Serialize, Deserialize};
use crate::util::artifact::{Artifact, ArtifactStatus};
use super::command_exists::CommandExists;

#[async_trait::async_trait]
pub trait CheckerOperations {

    async fn check(&self, artifact : &Artifact) -> Result<ArtifactStatus, anyhow::Error>;

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Checker {
    AcceptAll,
    Noop,
    CommandExists(CommandExists),
    Unknown
}

impl Checker {

    pub fn command_exists(command : String) -> Self {
        Checker::CommandExists(CommandExists(command))
    }

}

#[async_trait::async_trait]
impl CheckerOperations for Checker {

    async fn check(&self, artifact : &Artifact) -> Result<ArtifactStatus, anyhow::Error> {
        match self {
            Checker::AcceptAll => {
                Ok(ArtifactStatus::Installed)
            },
            Checker::CommandExists(command_exists) => {
                command_exists.check(artifact).await
            },
            _ => {
                Ok(ArtifactStatus::Unknown)
            }
        }
    }

}