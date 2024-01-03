use std::collections::{BTreeMap, BTreeSet};
use super::{
    KnownArtifact,
    ArtifactDependency,
    Artifact
};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::util::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ResolutionStatus {
    User,
    Resolved,
    Forced,

    // to be used when querying for artifacts
    Any
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub struct ArtifactResolution {
    pub artifact : Artifact,
    pub status : ResolutionStatus
}

impl ArtifactResolution {

    pub fn is_any(&self) -> bool {
        match self.status {
            ResolutionStatus::Any => true,
            _ => false
        }
    }

}

impl From<ArtifactResolution> for Artifact {
    fn from(artifact_installation : ArtifactResolution) -> Self {
        artifact_installation.artifact
    }
}

impl PartialEq for ArtifactResolution {
    fn eq(&self, other: &Self) -> bool {
        match self.status {
            ResolutionStatus::Any => self.artifact == other.artifact,
            _ => {
                self.status == other.status 
                && self.artifact == other.artifact
            }
        }
    }
}

impl Eq for ArtifactResolution {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactSet(BTreeMap<KnownArtifact, BTreeSet<ArtifactResolution>>);

impl ArtifactSet {

    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Inserts a particular artifact resolution into the resolution set.
    pub fn insert(&mut self, artifact : ArtifactResolution) {
        let artifact_set = self.0.entry(artifact.artifact.known_artifact.clone()).or_insert_with(|| BTreeSet::new());
        artifact_set.insert(artifact);
    }

    /// Removes a particular artifact from the resolution set.
    pub fn remove(&mut self, artifact : &ArtifactResolution) {
        let artifact_set = self.0.entry(artifact.artifact.known_artifact.clone()).or_insert_with(|| BTreeSet::new());
        artifact_set.remove(artifact);
    }

    /// Gets the resolved dependencies of a particular artifact.
    pub fn resolved_dependencies_of(&self, dependency : &ArtifactDependency) -> BTreeSet<&ArtifactResolution> {

        match self.0.get(&dependency.known_artifact()) {
            Some(resolution_set) => {
                resolution_set.iter().filter(|artifact_installation| {
                    dependency.compare(&artifact_installation.artifact)
                }).collect()
            },
            None => BTreeSet::new()
        }

    }

}