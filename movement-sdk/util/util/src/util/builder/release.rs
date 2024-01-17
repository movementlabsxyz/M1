use std::path::PathBuf;

use super::{Builder, BuilderOperations};
use crate::util::{
    artifact::{
        Artifact,
    },  
    release::ReleaseOperations,
    location::Location
};
use serde::{Serialize, Deserialize};
use crate::util::util::fs;
use crate::movement_dir::MovementDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Release;

impl Release {

    pub fn new() -> Self {
        Self {}
    }

}

#[async_trait::async_trait]
impl BuilderOperations for Release {

    async fn build(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        // todo: change this so that location always has a MovementDir modifier
        let path  = match &artifact.location {
            Location::Path(path) => {
                Ok::<PathBuf, anyhow::Error>(movement.path.join(path))
            },
            _ => {
                anyhow::bail!("Failed to build artifact.");
            }
        }?;

        // mkdir -p
        match path.parent() {
            Some(parent) => {
                tokio::fs::create_dir_all(parent).await?;
            },
            None => {
                anyhow::bail!("Failed to build artifact not located in parent dir.");
            }
        };

        artifact.release.get(&path.into()).await?;
        
        Ok(artifact.clone())

    }

    async fn remove(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        match &artifact.location {
            Location::Path(path) => {
                fs::remove(path).await?;
            },
            _ => {
                anyhow::bail!("Failed to remove artifact.")
            }
        };

        Ok(artifact.clone())

    }

}

impl Into<Builder> for Release {
    fn into(self) -> Builder {
        Builder::Release(self)
    }
}

#[cfg(test)]
pub mod test {

    #[tokio::test]
    async fn test_release() -> Result<(), anyhow::Error> {

        Ok(())

    }

}