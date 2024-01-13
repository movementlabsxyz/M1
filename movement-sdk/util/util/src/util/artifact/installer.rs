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
use crate::movement_dir::MovementDir;

#[async_trait::async_trait]
pub trait InstallerOperations {

    async fn resolve(
        &self, 
        movement_dir : &MovementDir,
        registry : &ArtifactRegistry
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error>;

    /// Installs the already resolved MovementDir
    async fn install_resolutions(
        &self,
        movement_dir : &MovementDir,
    ) -> Result<(), anyhow::Error>;

    /// Installs the 
    async fn install(
        &self,
        mut movement_dir : MovementDir,
        registry : &ArtifactRegistry
    ) -> Result<MovementDir, anyhow::Error> {

        let resolutions = self.resolve(&movement_dir, registry).await?;

        movement_dir.resolutions = resolutions;

        self.install_resolutions(&movement_dir).await?;

        Ok(movement_dir.clone())

    }

}

#[derive(Debug, Clone)]
pub struct BasicInstaller;

#[async_trait::async_trait]
impl InstallerOperations for BasicInstaller {

    async fn resolve(
        &self,
        movement_dir : &MovementDir,
        registry: &ArtifactRegistry
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error> {
    
        let mut resolutions = ArtifactDependencyResolutions::new();
        let mut queue = movement_dir.requirements.0.iter().cloned().collect::<Vec<_>>();

    
        while let Some(dependency) = queue.pop() {

            match movement_dir.resolutions.get(&dependency) {
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
        movement_dir : &MovementDir,
    ) -> Result<(), anyhow::Error> {

        // Handle uninstalls 
        let mut uninstalls = vec![];
        for (dependency, artifact) in movement_dir.resolutions.0.iter() {
            if !movement_dir.resolutions.resolved(dependency) {
                uninstalls.push(artifact);
            }
        }
        for artifact in uninstalls {
            artifact.uninstall(movement_dir).await?;
        }


        // Handle installs
        let resolutions_owned = movement_dir.resolutions.clone();
        let artifact_resolutions : ArtifactResolutions = resolutions_owned.try_into()?;
        let resolution_plan : ArtifactResolutionPlan = artifact_resolutions.try_into()?;

        for artifacts in resolution_plan.0 {

            let mut futures = vec![];

            for artifact in artifacts {

                let future = async move {

                    let artifact = artifact.clone();
                    match artifact.check().await? {
                        ArtifactStatus::Installed => {},
                        _ => artifact.install(movement_dir).await?
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

        let dir = tempfile::tempdir()?;
        let mut movement_dir = MovementDir::new(&dir.path().to_path_buf());

        let installer = BasicInstaller;
        let registry = ArtifactRegistry::InMemory(InMemoryArtifactRegistry::new());

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

        movement_dir.requirements.add(
            ArtifactDependency::identifier(
                KnownArtifact::Name("moons".to_string()),
                Version::new(0, 0, 0)
            )
        );

        let movement_dir = installer.install(movement_dir, &registry).await?;

        assert_eq!(movement_dir.resolutions.len(), 2);
      
        Ok(())

    }

}