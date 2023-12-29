use serde::{Serialize, Deserialize};
use super::known_script::KnownScript;
use crate::util::artifact::{Artifact, self};
use super::download_release::DownloadRelease;

#[async_trait::async_trait]
pub trait BuilderOperations {

    async fn build(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error>;

    async fn remove(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error>;

}

/// A builder performs the task of reifying an artifact.
/// It will use the information about the release, version, location, etc.
/// to build the artifact.
/// A builder should not deal with the artifact's dependencies. This is left to the artifact itself.
/// A builder should not deal with the artifact's checker, i.e., for idempotency. This is left to the artifact itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Builder {
    KnownScript(KnownScript), 
    RustBuild,
    FromArchive,
    DownloadRelease(DownloadRelease),
    Pipeline,
    Noop,
    Unknown
}

#[async_trait::async_trait]
impl BuilderOperations for Builder {

    async fn build(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        match self {
            Builder::KnownScript(script) => {
                script.build(artifact).await
            },
            Builder::RustBuild => {
                todo!()
            },
            Builder::FromArchive => {
                todo!()
            },
            Builder::DownloadRelease(release) => {
                todo!()
            },
            Builder::Pipeline => {
                todo!()
            },
            Builder::Noop => {
                Ok(artifact.clone())
            },
            _ => {
                anyhow::bail!("Cannot build an unsupported builder type.");
            }
        }

    }

    async fn remove(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        match self {
            Builder::KnownScript(script) => {
                script.remove(artifact).await
            },
            Builder::RustBuild => {
                todo!()
            },
            Builder::FromArchive => {
                todo!()
            },
            Builder::DownloadRelease(release) => {
                todo!()
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
