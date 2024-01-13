use super::{Artifact, ArtifactDependency, KnownArtifact};
use std::collections::{BTreeSet, BTreeMap};
use serde::{Serialize, Deserialize, Deserializer};
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

impl Default for InMemoryArtifactRegistry {

    fn default() -> Self {
        Self::new()
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
    InMemory(InMemoryArtifactRegistry),
}

impl ArtifactRegistry {

    pub fn to_string(&self) -> String {
        match self {
            Self::InMemory(_) => "in-memory".to_string()
        }
    }

    pub fn from_string(string : &str) -> Result<Self, anyhow::Error> {
        match string {
            "in-memory" => Ok(Self::InMemory(Default::default())),
            _ => anyhow::bail!("Unknown artifact registry: {}", string)
        }
    }

}

impl Serialize for ArtifactRegistry {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_str(&self.to_string())
    }

}

impl<'de> Deserialize<'de> for ArtifactRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> 
    where 
        D: Deserializer<'de>, 
    {
        let string = String::deserialize(deserializer)?;
        Self::from_string(&string).map_err(serde::de::Error::custom)
    }
}

impl PartialEq for ArtifactRegistry {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for ArtifactRegistry {}

#[async_trait::async_trait]
impl ArtifactRegistryOperations for ArtifactRegistry {

    async fn find(&self, dependency : &ArtifactDependency) -> Result<Option<Artifact>, anyhow::Error> {
        match self {
            Self::InMemory(registry) => registry.find(dependency).await
        }
    }

    async fn register(&self, artifact : &Artifact) -> Result<(), anyhow::Error> {
        match self {
            Self::InMemory(registry) => registry.register(artifact).await
        }
    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use crate::util::util::Version;


    #[tokio::test]
    pub async fn test_in_memory_artifact_registry() -> Result<(), anyhow::Error> {

        let registry = InMemoryArtifactRegistry::new();

        let moon = "moon";
        let moon_known_artifact = KnownArtifact::Name(moon.to_string());
        let moon_version = Version::new(1, 0, 0);
        let moon_available_version = Version::new(1, 0, 1);

        let moon_dep = ArtifactDependency::identifier(
            moon_known_artifact.clone(),
            moon_version.clone()
        );

        let artifact = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_available_version.clone());

        registry.register(&artifact).await?;

        let found = registry.find(&moon_dep).await?;

        assert_eq!(found, Some(artifact));

        Ok(())

    }

    #[tokio::test]
    pub async fn test_in_memory_artifact_registry_should_fail() -> Result<(), anyhow::Error> {

        let registry = InMemoryArtifactRegistry::new();

        let moon = "moon";
        let moon_known_artifact = KnownArtifact::Name(moon.to_string());
        let moon_version = Version::new(1, 0, 0);
    
        let moon_dep = ArtifactDependency::identifier(
            moon_known_artifact.clone(),
            moon_version.clone()
        );

        let found = registry.find(&moon_dep).await?;

        assert_eq!(found, None);

        let moon_minor_bump_version = Version::new(1, 1, 0);
        let moon_minor_bump = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_minor_bump_version.clone());
        registry.register(&moon_minor_bump).await?; 

        let found = registry.find(&moon_dep).await?;
        assert_eq!(found, None);

        let moon_major_bump_version = Version::new(2, 0, 0);
        let moon_major_bump = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_major_bump_version.clone());
        registry.register(&moon_major_bump).await?;

        let found = registry.find(&moon_dep).await?;
        assert_eq!(found, None);

        Ok(())

    }

    #[tokio::test]
    pub async fn test_in_memory_artifact_registry_picks_one_deterministically() -> Result<(), anyhow::Error> {

        let registry = InMemoryArtifactRegistry::new();

        let moon = "moon";
        let moon_known_artifact = KnownArtifact::Name(moon.to_string());
        let moon_version = Version::new(1, 0, 0);
        let moon_available_version_1 = Version::new(1, 0, 1);
        let moon_available_version_2 = Version::new(1, 0, 2);
        let moon_available_version_3 = Version::new(2, 0, 1);
        let moon_available_version_4 = Version::new(1, 1, 2);

        let moon_dep = ArtifactDependency::identifier(
            moon_known_artifact.clone(),
            moon_version.clone()
        );

        let artifact_1 = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_available_version_1.clone());
        registry.register(&artifact_1).await?;

        let artifact_2 = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_available_version_2.clone());
        registry.register(&artifact_2).await?;

        let artifact_3 = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_available_version_3.clone());
        registry.register(&artifact_3).await?;

        let artifact_4 = Artifact::test()
        .with_name(moon.to_string())
        .with_version(moon_available_version_4.clone());

        let found_once = registry.find(&moon_dep).await?;
        let found_again = registry.find(&moon_dep).await?;

        assert_eq!(found_once, found_again);
        assert_ne!(found_once, Some(artifact_3));
        assert_ne!(found_once, Some(artifact_4));

        Ok(())

    }

}