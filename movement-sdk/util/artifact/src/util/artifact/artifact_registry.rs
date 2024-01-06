use super::{Artifact, ArtifactDependency, KnownArtifact};
use std::collections::{BTreeSet, BTreeMap};
use tokio::sync::RwLock;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait ArtifactRegistryOperations {

    async fn find(&self, artifact : &ArtifactDependency) -> Result<Option<Artifact>, anyhow::Error>;

    async fn register(&self, artifact : &Artifact) -> Result<(), anyhow::Error>;

}

#[derive(Debug, Clone)]
pub struct InMemoryArtifactRegistry {
    pub artifacts : Arc<RwLock<BTreeMap<KnownArtifact, BTreeSet<Artifact>>>>
}

impl InMemoryArtifactRegistry {

    pub fn new() -> Self {
        Self {
            artifacts : Arc::new(RwLock::new(BTreeMap::new()))
        }
    }

}

#[async_trait::async_trait]
impl ArtifactRegistryOperations for InMemoryArtifactRegistry {

    async fn find(&self, dependency : &ArtifactDependency) -> Result<Option<Artifact>, anyhow::Error> {
        
        let known_artifact = dependency.known_artifact();

        let artifacts = self.artifacts.read().await;

        match artifacts.get(&known_artifact) {
            Some(artifacts) => {
                for artifact in artifacts {
                    if dependency.compare(artifact) {
                        return Ok(Some(artifact.clone()))
                    }
                }
                Ok(None)
            },
            None => Ok(None)
        }

    }

    async fn register(&self, artifact : &Artifact) -> Result<(), anyhow::Error> {

        let known_artifact = artifact.known_artifact.clone();

        let mut artifacts = self.artifacts.write().await;

        let artifact_set = artifacts.entry(known_artifact).or_insert_with(|| BTreeSet::new());
        artifact_set.insert(artifact.clone());

        Ok(())

    }

}

#[derive(Debug, Clone)]
pub enum ArtifactRegistry {
    InMemory(InMemoryArtifactRegistry)
}

pub mod known {

    use super::*;
    use crate::util::artifact::artifacts::{
        cli,
        m1
    };

    #[async_trait::async_trait]
    pub trait Known {

        async fn known() -> Result<Self, anyhow::Error>
        where Self : Sized;

    }

    #[async_trait::async_trait]
    impl Known for InMemoryArtifactRegistry {
        async fn known() -> Result<Self, anyhow::Error> {
            let mut known = Self::new();

            // ! Register your known artifacts below

            Ok(known)
        }
    }

    #[async_trait::async_trait]
    impl Known for ArtifactRegistry {
        async fn known() -> Result<Self, anyhow::Error> {
            Ok(Self::InMemory(InMemoryArtifactRegistry::known().await?))
        }
    }

}
