use serde::{Serialize, Deserialize};
use crate::util::builder::{Builder, BuilderOperations};
use crate::util::location::Location;
use crate::util::release::Release;
use crate::util::util::{Version, version::VersionTolerance};
use crate::util::checker::{Checker, CheckerOperations};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KnownArtifact {
    // general
    Unknown,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactIdentifierPartial(
    pub KnownArtifact,
    pub Version
);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactIdentifierFull(
    pub KnownArtifact,
    pub Version,
    pub VersionTolerance
);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactIdentifier {
    Partial(ArtifactIdentifierPartial),
    Full(ArtifactIdentifierFull)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactDependency {
    Artifact(Artifact),
    ArtifactIdentifier(ArtifactIdentifier)
}

// Implement From<Artifact> for ArtifactDependency
impl From<Artifact> for ArtifactDependency {
    fn from(artifact: Artifact) -> Self {
        ArtifactDependency::Artifact(artifact)
    }
}

// Implement From<ArtifactIdentifier> for ArtifactDependency
impl From<ArtifactIdentifier> for ArtifactDependency {
    fn from(artifact_identifier: ArtifactIdentifier) -> Self {
        ArtifactDependency::ArtifactIdentifier(artifact_identifier)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash,PartialOrd, Ord)]
pub struct Artifact {
    pub release : Release,
    pub location : Location,
    pub version : Version,
    pub builder : Builder,
    pub checker : Checker,
    pub dependencies : BTreeSet<ArtifactDependency>
}

impl Artifact {

    pub fn new(
        release : Release, 
        location : Location, 
        version : Version,
        builder : Builder, 
        checker : Checker,
        dependencies : BTreeSet<ArtifactDependency>
    ) -> Self {
        Self {
            release,
            location,
            version,
            builder,
            checker,
            dependencies
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ArtifactStatus {
    Unknown,
    Installing,
    Installed,
    Uninstalling,
    Broken
}