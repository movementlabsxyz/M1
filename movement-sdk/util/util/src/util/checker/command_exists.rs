use serde::{Serialize, Deserialize};
use crate::util::artifact::{Artifact, ArtifactStatus};
use super::CheckerOperations;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommandExists(pub String);

#[async_trait::async_trait]
impl CheckerOperations for CommandExists {

    async fn check(&self, artifact : &Artifact) -> Result<ArtifactStatus, anyhow::Error> {
        let command = &self.0;
        let command_exists = which::which(command).is_ok();
        if command_exists {
            Ok(ArtifactStatus::Installed)
        } else {
            Ok(ArtifactStatus::Unknown)
        }
    }

}