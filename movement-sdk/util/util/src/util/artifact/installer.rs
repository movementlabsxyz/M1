use super::requirements::ArtifactRequirements;
use super::resolution::{
    ArtifactResolutionPlan,
    ArtifactResolutions,
    ArtifactDependencyResolutions
};
use super::registry::{
    ArtifactRegistry,
    ArtifactRegistryOperations
};
use super::ArtifactStatus;
use anyhow::anyhow;

#[async_trait::async_trait]
pub trait InstallerOperations {

    async fn resolve(
        &self, 
        requirements : &ArtifactRequirements,
        lock : &ArtifactDependencyResolutions,
        registry : &ArtifactRegistry
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error>;

    async fn install_resolutions(
        &self,
        lock : &ArtifactDependencyResolutions,
        resolutions : &ArtifactDependencyResolutions,
    ) -> Result<(), anyhow::Error>;

    async fn install(
        &self,
        requirements : &ArtifactRequirements,
        lock : &ArtifactDependencyResolutions,
        registry : &ArtifactRegistry
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error> {

        let resolutions = self.resolve(requirements, lock, registry).await?;

        self.install_resolutions(lock, &resolutions).await?;

        Ok(resolutions)

    }

}

#[derive(Debug, Clone)]
pub struct BasicInstaller;

#[async_trait::async_trait]
impl InstallerOperations for BasicInstaller {

    async fn resolve(
        &self,
        requirements: &ArtifactRequirements,
        lock: &ArtifactDependencyResolutions,
        registry: &ArtifactRegistry
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error> {
    
        let mut resolutions = ArtifactDependencyResolutions::new();
        let mut queue = requirements.0.iter().cloned().collect::<Vec<_>>();

    
        while let Some(dependency) = queue.pop() {

            match lock.get(&dependency) {
                Some(artifact) => {
                    resolutions.add(dependency.clone(), artifact.clone());
                },
                None => {
                    match registry.find(&dependency).await? {
                        Some(artifact) => {
                            for inner_dependency in &artifact.dependencies {
                                queue.push(inner_dependency.clone());
                            }
                            resolutions.add(dependency, artifact);
                        },
                        None => {
                            return Err(anyhow!("Could not find artifact for dependency: {:?}", dependency));
                        }
                    }
                }
            }
    
        }
    
        Ok(resolutions)

    }

    async fn install_resolutions(
        &self,
        lock : &ArtifactDependencyResolutions,
        resolutions : &ArtifactDependencyResolutions,
    ) -> Result<(), anyhow::Error> {

        // Handle uninstalls 
        let mut uninstalls = vec![];
        for (dependency, artifact) in lock.0.iter() {
            if !resolutions.resolved(dependency) {
                uninstalls.push(artifact);
            }
        }
        for artifact in uninstalls {
            artifact.uninstall().await?;
        }


        // Handle installs
        let resolutions_owned = resolutions.clone();
        let artifact_resolutions : ArtifactResolutions = resolutions_owned.try_into()?;
        let resolution_plan : ArtifactResolutionPlan = artifact_resolutions.try_into()?;

        for artifacts in resolution_plan.0 {

            let mut futures = vec![];

            for artifact in artifacts {

                let future = async move {

                    let artifact = artifact.clone();
                    match artifact.check().await? {
                        ArtifactStatus::Installed => {},
                        _ => artifact.install().await?
                    };

                    Ok::<(), anyhow::Error>(())

                };
                futures.push(future);

            }

            futures::future::try_join_all(futures).await?;

        }

        Ok(())

    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use crate::artifact::registry::InMemoryArtifactRegistry;
    use crate::artifact::{Artifact, ArtifactDependency, KnownArtifact};
    use crate::util::util::Version;

    #[tokio::test]
    pub async fn test_install() -> Result<(), anyhow::Error> {

        let installer = BasicInstaller;
        let registry = ArtifactRegistry::InMemory(InMemoryArtifactRegistry::new());
        let mut requirements = ArtifactRequirements::new();

        let stars_v0 = Artifact::test().with_name("stars".to_string()).with_version(Version::new(0, 0, 0));
        let stars_v0_1 = Artifact::test().with_name("stars".to_string()).with_version(Version::new(0, 0, 1));

        let moons_v0 = Artifact::test()
        .with_name("moons".to_string())
        .with_version(Version::new(0, 0, 0))
        .with_dependencies(
            vec![
                ArtifactDependency::identifier(
                    KnownArtifact::Name("stars".to_string()),
                    Version::new(0, 0, 0)
                )
            ].into_iter().collect()
        );

        registry.register(&stars_v0).await?;
        registry.register(&stars_v0_1).await?;
        registry.register(&moons_v0).await?;

        requirements.add(
            ArtifactDependency::identifier(
                KnownArtifact::Name("moons".to_string()),
                Version::new(0, 0, 0)
            )
        );

        let lock = installer.install(&requirements, &ArtifactDependencyResolutions::new(), &registry).await?;

        assert_eq!(lock.len(), 2);
      
        Ok(())

    }

}