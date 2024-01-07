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
        let mut queue = requirements.0.iter().collect::<Vec<_>>();
    
        while let Some(dependency) = queue.pop() {
            // Check if the dependency is already resolved
            if lock.resolved(dependency) {
                resolutions.add(dependency.clone(), lock.0.get(dependency).unwrap().clone());
            } else {
                // Otherwise, find the latest version of the dependency
                match registry.find(dependency).await? {
                    Some(artifact) => {
                        resolutions.add(dependency.clone(), artifact);
                    },
                    None => {
                        return Err(anyhow!("Could not find artifact for dependency: {:?}", dependency));
                    }
                }
            }
    
            // Add the dependencies of the artifact to the queue
            if let Some(artifact) = lock.0.get(dependency) {
                queue.extend(artifact.dependencies.iter());
            } else {
                return Err(anyhow!("Could not find artifact for dependency: {:?}", dependency));
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