use serde::{Serialize, Deserialize};
use crate::util::builder::{self, Builder, BuilderOperations};
use crate::util::location::Location;
use crate::util::release::Release;
use crate::util::util::{Version, version::VersionTolerance};
use crate::util::checker::{Checker, CheckerOperations};
use std::collections::BTreeSet;
use std::fmt::Display;
use std::path::PathBuf;
use crate::movement_dir::MovementDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KnownArtifact {

    // general
    Unknown,
    Test,

    // generalized
    Name(String)
}

impl Display for KnownArtifact {
    
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnownArtifact::Unknown => write!(f, "unknown"),
            KnownArtifact::Test => write!(f, "test"),
            KnownArtifact::Name(name) => write!(f, "{}", name)
        }
    }

}

impl Into<String> for KnownArtifact {
    fn into(self) -> String {
        match self {
            KnownArtifact::Unknown => "unknown".to_string(),
            KnownArtifact::Test => "test".to_string(),
            KnownArtifact::Name(name) => name
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactIdentifierPartial(
    pub KnownArtifact,
    pub Version
);

impl ArtifactIdentifierPartial {

    pub fn new(known_artifact : KnownArtifact, version : Version) -> Self {
        Self(known_artifact, version)
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactIdentifierFull(
    pub KnownArtifact,
    pub Version,
    pub VersionTolerance
);

impl ArtifactIdentifierFull {

    pub fn new(known_artifact : KnownArtifact, version : Version, version_tolerance : VersionTolerance) -> Self {
        Self(known_artifact, version, version_tolerance)
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactIdentifier {
    Partial(ArtifactIdentifierPartial),
    Full(ArtifactIdentifierFull)
}

impl ArtifactIdentifier {

    pub fn new(known_artifact : KnownArtifact, version : Version) -> Self {
        Self::Partial(ArtifactIdentifierPartial::new(known_artifact, version))
    }

    pub fn known_artifact(&self) -> KnownArtifact {
        match self {
            ArtifactIdentifier::Partial(partial) => partial.0.clone(),
            ArtifactIdentifier::Full(full) => full.0.clone()
        }
    }

    pub fn version(&self) -> Version {
        match self {
            ArtifactIdentifier::Partial(partial) => partial.1.clone(),
            ArtifactIdentifier::Full(full) => full.1.clone()
        }
    }

    pub fn version_tolerance(&self) -> VersionTolerance {
        match self {
            ArtifactIdentifier::Partial(_) => VersionTolerance::default(),
            ArtifactIdentifier::Full(full) => full.2.clone()
        }
    }

    pub fn compare(&self, artifact : &Artifact) -> bool {

        let known_artifact = self.known_artifact();
        let version = self.version();
        let version_tolerance = self.version_tolerance();

        if known_artifact != artifact.known_artifact {
            return false;
        }

        version_tolerance.permits(&version, &artifact.version)

    }

}

impl Display for ArtifactIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactIdentifier::Partial(partial) => write!(f, "{}={}", partial.0, partial.1),
            ArtifactIdentifier::Full(full) => write!(f, "{}={}", full.0, full.1)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactDependency {
    /// Either a particular artifact.
    Artifact(Artifact),
    /// Or defined artifact version.
    ArtifactIdentifier(ArtifactIdentifier)
}

impl ArtifactDependency {

    pub fn identifier(known_artifact : KnownArtifact, version : Version) -> Self {
        Self::ArtifactIdentifier(ArtifactIdentifier::new(known_artifact, version))
    }

    pub fn known_artifact(&self) -> KnownArtifact {
        match self {
            ArtifactDependency::Artifact(artifact) => artifact.known_artifact.clone(),
            ArtifactDependency::ArtifactIdentifier(artifact_identifier) => artifact_identifier.known_artifact()
        }
    }

    pub fn compare(&self, comparison_artifact : &Artifact) -> bool {

        match self {
            ArtifactDependency::Artifact(artifact) => artifact == comparison_artifact,
            ArtifactDependency::ArtifactIdentifier(artifact_identifier) => artifact_identifier.compare(comparison_artifact)
        }

    }

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

impl Display for ArtifactDependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactDependency::Artifact(artifact) => write!(f, "{}", artifact),
            ArtifactDependency::ArtifactIdentifier(artifact_identifier) => write!(f, "{}", artifact_identifier)
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash,PartialOrd, Ord)]
pub struct Artifact {
    pub known_artifact : KnownArtifact,
    pub release : Release,
    pub location : Location,
    pub version : Version,
    pub builder : Builder,
    pub checker : Checker,
    pub dependencies : BTreeSet<ArtifactDependency>
}

impl Artifact {

    pub fn new(
        known_artifact : KnownArtifact,
        release : Release, 
        location : Location, 
        version : Version,
        builder : Builder, 
        checker : Checker,
        dependencies : BTreeSet<ArtifactDependency>
    ) -> Self {
        Self {
            known_artifact,
            release,
            location,
            version,
            builder,
            checker,
            dependencies
        }
    }

    pub fn test() -> Self {
        Self {
            known_artifact : KnownArtifact::Test,
            release : Release::Noop,
            location : Location::Unknown,
            version : Version::Latest,
            builder : Builder::Noop,
            checker : Checker::Noop,
            dependencies : BTreeSet::new()
        }
    }

    pub fn with_name(mut self, name : String) -> Self {
        self.known_artifact = KnownArtifact::Name(name);
        self
    }

    pub fn with_dependencies(mut self, dependencies : BTreeSet<ArtifactDependency>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_version(mut self, version : Version) -> Self {
        self.version = version;
        self
    }

    pub async fn install(&self, movement : &MovementDir) -> Result<(), anyhow::Error> {

        self.builder.build(&self, movement).await?;
        Ok(())

    }

    pub async fn uninstall(&self, movement : &MovementDir) -> Result<(), anyhow::Error> {
        
        self.builder.remove(&self, movement).await?;
        Ok(())

    }

    pub async fn check(&self) -> Result<ArtifactStatus, anyhow::Error> {
        self.checker.check(&self).await
    }

    pub fn self_contained_script(name : String, script : String) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name),
            release : Release::Noop,
            location : Location::Unknown,
            version : Version::Latest,
            builder : Builder::Script(script.into()),
            checker : Checker::Noop,
            dependencies : BTreeSet::new()
        }
    }

    pub fn noop(name : String) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name),
            release : Release::Noop,
            location : Location::Unknown,
            version : Version::Latest,
            builder : Builder::Noop,
            checker : Checker::Noop,
            dependencies : BTreeSet::new()
        }
    }

    pub fn unsupported(name : String) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name),
            release : Release::Noop,
            location : Location::Unknown,
            version : Version::Latest,
            builder : Builder::Unsupported,
            checker : Checker::Noop,
            dependencies : BTreeSet::new()
        }
    }

    pub fn source_tar_gz_release(name : String, release : Release) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name.clone()),
            release,
            location : PathBuf::from("src").into(),
            version : Version::Latest,
            builder : Builder::Unarchive(builder::unarchive::Unarchive::TarGz),
            checker : Checker::Noop,
            dependencies : BTreeSet::new()
        }
    }

    pub fn resource_release(name : String, release : Release) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name.clone()),
            release,
            location : PathBuf::from("rsc").join(name).into(),
            version : Version::Latest,
            builder : Builder::Release(builder::release::Release::new()),
            checker : Checker::Noop,
            dependencies : BTreeSet::new()
        }
    }

    pub fn bin_release(name : String, release : Release) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name.clone()),
            release,
            location : PathBuf::from("bin").join(name.clone()).into(),
            version : Version::Latest,
            builder : Builder::Release(builder::release::Release::new()),
            checker : Checker::command_exists(name),
            dependencies : BTreeSet::new()
        }
    }

    pub fn pessimistic_bin_release(name : String, release : Release) -> Self {
        Self {
            known_artifact : KnownArtifact::Name(name.clone()),
            release,
            location : PathBuf::from("bin").join(name.clone()).into(),
            version : Version::Latest,
            builder : Builder::Release(builder::release::Release::new()),
            checker : Checker::command_exists(name),
            dependencies : BTreeSet::new()
        }
    }

    pub fn with_checker(mut self, checker : Checker) -> Self {
        self.checker = checker;
        self
    }

}

impl Display for Artifact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.known_artifact, self.version)
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