use serde::{Serialize, Deserialize};
use crate::util::builder::{Builder, BuilderOperations};
use crate::util::location::Location;
use crate::util::release::Release;
use crate::util::util::Version;
use crate::util::checker::{Checker, CheckerOperations};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub release : Release,
    pub location : Location,
    pub version : Version,
    pub builder : Builder,
    pub checker : Checker
}

impl Artifact {

    pub fn new(release : Release, location : Location, version : Version, builder : Builder, checker : Checker) -> Self {
        Self {
            release,
            location,
            version,
            builder,
            checker
        }
    }

    async fn install(&self) -> Result<(), anyhow::Error> {

        self.builder.build(&self).await?;
        Ok(())

    }

    async fn uninstall(&self) -> Result<(), anyhow::Error> {
        
        self.builder.remove(&self).await?;
        Ok(())

    }

    async fn check(&self) -> Result<ArtifactStatus, anyhow::Error> {
        self.checker.check(&self).await
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactStatus {
    Unknown,
    Installing,
    Installed,
    Uninstalling,
    Broken
}