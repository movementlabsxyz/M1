use serde::{Serialize, Deserialize};
use super::script::Script;
use crate::util::artifact::Artifact;
use super::release::Release;
use super::unarchive::Unarchive;
use crate::movement_dir::MovementDir;

#[async_trait::async_trait]
pub trait BuilderOperations {

    async fn build(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error>;

    async fn remove(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error>;

}

/// A builder performs the task of reifying an artifact.
/// It will use the information about the release, version, location, etc.
/// to build the artifact.
/// A builder should not deal with the artifact's dependencies. This is left to the artifact itself.
/// A builder should not deal with the artifact's checker, i.e., for idempotency. This is left to the artifact itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Builder {
    Script(Script), 
    RustBuild,
    Unarchive(Unarchive),
    Release(Release),
    Pipeline,
    Unsupported,
    Noop,
    Unknown
}

#[async_trait::async_trait]
impl BuilderOperations for Builder {

    async fn build(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        match self {
            Builder::Script(script) => {
                script.build(artifact, movement).await
            },
            Builder::RustBuild => {
                todo!()
            },
            Builder::Unarchive(unarchive) => {
                unarchive.build(artifact, movement).await
            },
            Builder::Release(release) => {
                release.build(artifact, movement).await
            },
            Builder::Pipeline => {
                todo!()
            },
            Builder::Unsupported => {
                let name : String = artifact.known_artifact.clone().into();
                anyhow::bail!(
                    format!("Builder does not support artifact: {:?}", name)
                );
            },
            Builder::Noop => {
                Ok(artifact.clone())
            },
            _ => {
                anyhow::bail!("Cannot build an unsupported builder type.");
            }
        }

    }

    async fn remove(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        match self {
            Builder::Script(script) => {
                script.remove(artifact, movement).await
            },
            Builder::RustBuild => {
                todo!()
            },
            Builder::Unarchive(unarchive) => {
                unarchive.remove(artifact, movement).await
            },
            Builder::Release(release) => {
                release.remove(artifact, movement).await
            },
            Builder::Pipeline => {
                todo!()
            },
            Builder::Noop => {
                Ok(artifact.clone())
            },
            _ => {
                anyhow::bail!("Cannot remove an unsupported builder type.");
            }
            
        }

    }

}
