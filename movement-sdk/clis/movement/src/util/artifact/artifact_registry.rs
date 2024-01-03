use super::{Artifact, ArtifactDependency};

#[async_trait::async_trait]
pub trait ArtifactRegistryOperations {

    async fn find(&self, artifact : &ArtifactDependency) -> Result<Artifact, anyhow::Error>;

}