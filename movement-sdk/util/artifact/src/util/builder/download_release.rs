use super::{Builder, BuilderOperations};
use crate::util::{
    artifact::Artifact,  
    release::ReleaseOperations
};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DownloadRelease;

impl DownloadRelease {

    pub fn new() -> Self {
        Self {}
    }

}

#[async_trait::async_trait]
impl BuilderOperations for DownloadRelease {

    async fn build(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        artifact.release.get(&artifact.location).await?;
        
        Ok(artifact.clone())

    }

    async fn remove(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        Ok(artifact.clone())

    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use crate::util::{
        artifact::Artifact, 
        release::Release, 
        location::Location,  
        util::Version,
        builder::Builder,
        checker::Checker
    };
    use std::collections::BTreeSet;

    #[tokio::test]
    async fn test_known_script() -> Result<(), anyhow::Error> {

        Ok(())

    }

}

impl Into<Builder> for DownloadRelease {
    fn into(self) -> Builder {
        Builder::DownloadRelease(self)
    }
}