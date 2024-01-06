use std::collections::{BTreeMap, BTreeSet};
use super::{
    KnownArtifact,
    ArtifactDependency,
    Artifact
};
use serde::{Serialize, Deserialize};

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

/*#[async_trait::async_trait]
impl ArtifactRegistryOperations for ArtifactSet {

    async fn find(&self, dependency : &ArtifactDependency) -> Result<Option<Artifact>, anyhow::Error> {
        
        let known_artifact = dependency.known_artifact();

        match self.0.get(&known_artifact) {
            Some(artifacts) => {
                for artifact in artifacts {
                    if dependency.compare(&artifact.artifact) {
                        return Ok(Some(artifact.artifact.clone()))
                    }
                }
                Ok(None)
            },
            None => Ok(None)
        }

    }

    async fn register(&self, artifact : &Artifact) -> Result<(), anyhow::Error> {

        let known_artifact = artifact.known_artifact.clone();

        let artifact_set = self.0.entry(known_artifact.clone()).or_insert_with(|| BTreeSet::new());
        artifact_set.insert(ArtifactResolution {
            artifact : artifact.clone(),
            status : ResolutionStatus::User
        });

        Ok(())

    }

}*/

pub mod asynchronous {


    use super::{
        ArtifactSet as ArtifactSetSync,
        ArtifactResolution,
        ResolutionStatus,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use super::super::{
        KnownArtifact,
        ArtifactDependency,
        Artifact
    };
    use super::super::artifact_registry::ArtifactRegistryOperations;
    use serde::{Serialize, Deserialize};
    use tokio::sync::RwLock;
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    pub struct ArtifactSet(Arc<RwLock<ArtifactSetSync>>);

    impl ArtifactSet {

        pub fn new() -> Self {
            Self(Arc::new(RwLock::new(ArtifactSetSync::new())))
        }

        /// Inserts a particular artifact resolution into the resolution set.
        pub async fn insert(&self, artifact : ArtifactResolution) {
            let mut artifact_set = self.0.write().await;
            artifact_set.insert(artifact);
        }

        /// Removes a particular artifact from the resolution set.
        pub async fn remove(&self, artifact : &ArtifactResolution) {
            let mut artifact_set = self.0.write().await;
            artifact_set.remove(artifact);
        }

    }

    #[async_trait::async_trait]
    impl ArtifactRegistryOperations for ArtifactSet {

        async fn find(&self, dependency : &ArtifactDependency) -> Result<Option<Artifact>, anyhow::Error> {
            
            let known_artifact = dependency.known_artifact();

            let artifact_set = self.0.read().await;

            match artifact_set.0.get(&known_artifact) {
                Some(artifacts) => {
                    for artifact in artifacts {
                        if dependency.compare(&artifact.artifact) {
                            return Ok(Some(artifact.artifact.clone()))
                        }
                    }
                    Ok(None)
                },
                None => Ok(None)
            }

        }

        async fn register(&self, artifact : &Artifact) -> Result<(), anyhow::Error> {

            let known_artifact = artifact.known_artifact.clone();

            let mut artifact_set = self.0.write().await;

            artifact_set.insert(ArtifactResolution {
                artifact : artifact.clone(),
                status : ResolutionStatus::User
            });

            Ok(())

        }

    }

}