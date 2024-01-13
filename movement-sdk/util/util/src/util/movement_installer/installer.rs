use crate::util::movement_dir::MovementDir;
use crate::util::artifact::{
    registry::ArtifactRegistry,
    ArtifactDependency,
    resolution::ArtifactDependencyResolutions,
    installer::{BasicInstaller, InstallerOperations as ArtifactInstallerOperations}
};

#[async_trait::async_trait]
pub trait MovementInstallerOperations {

    async fn install_resolve(&self, 
        movement_dir : MovementDir, 
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error>;

    async fn uninstall_resolve(&self, 
        movement_dir : MovementDir,
        registry : &ArtifactRegistry, 
        dependencies : Vec<ArtifactDependency>
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error>;

    async fn install_resolutions(
        &self, 
        movement_dir : MovementDir, 
        resolutions : ArtifactDependencyResolutions
    ) -> Result<MovementDir, anyhow::Error>;

    async fn install(&self, 
        movement_dir : MovementDir, 
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<MovementDir, anyhow::Error>;

    async fn uninstall(
        &self, 
        movement_dir : MovementDir,
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<MovementDir, anyhow::Error>;

}

#[derive(Debug, Clone)]
pub struct MovementInstaller {
    pub basic_installer : BasicInstaller,
}


#[async_trait::async_trait]
impl MovementInstallerOperations for MovementInstaller {

    async fn install_resolve(&self, 
        movement_dir : MovementDir, 
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error> {

        let mut movement_dir = movement_dir.load()?;

        // add all of the dependencies
        for dependency in dependencies {
            movement_dir.requirements.add(dependency);
        }

        // resolve the dependencies
        let resolutions = self.basic_installer.resolve(
            &movement_dir,
            registry
        ).await?;

        Ok(resolutions)

    }

    async fn uninstall_resolve(
        &self, 
        movement_dir : MovementDir, 
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<ArtifactDependencyResolutions, anyhow::Error> {

        let mut movement_dir = movement_dir.load()?;

        // remove all of the dependencies
        for dependency in dependencies {
            movement_dir.requirements.remove(&dependency);
        }

        // resolve the dependencies
        let resolutions = self.basic_installer.resolve(
            &movement_dir,
            registry
        ).await?;

        Ok(resolutions)

    }

    async fn install_resolutions(&self, movement_dir : MovementDir, resolutions : ArtifactDependencyResolutions) -> Result<MovementDir, anyhow::Error> {

        let mut movement_dir = movement_dir.load()?;

        // install the resolutions
        self.basic_installer.install_resolutions(
          &movement_dir
        ).await?;

        movement_dir.resolutions = resolutions;

        movement_dir.store()?;

        Ok(movement_dir)

    }

    async fn install(
        &self, 
        movement_dir : MovementDir, 
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<MovementDir, anyhow::Error> {

        let mut movement_dir = movement_dir.load()?;

        // add all of the dependencies
        for dependency in dependencies {
            movement_dir.requirements.add(dependency);
        }

        // resolve the dependencies
        let movement_dir = self.basic_installer.install(
            movement_dir,
            registry
        ).await?;

        movement_dir.store()?;

        Ok(movement_dir)

    }

    async fn uninstall(
        &self, 
        movement_dir : MovementDir, 
        registry : &ArtifactRegistry,
        dependencies : Vec<ArtifactDependency>
    ) -> Result<MovementDir, anyhow::Error> {

        let mut movement_dir = movement_dir.load()?;

        // remove all of the dependencies
        for dependency in dependencies {
            movement_dir.requirements.remove(&dependency);
        }

        // resolve the dependencies
        let movement_dir = self.basic_installer.install(
            movement_dir,
            registry
        ).await?;

        movement_dir.store()?;

        Ok(movement_dir)

    }

}

#[cfg(test)]
pub mod test {


    use super::*;
    use crate::util::artifact::{
        Artifact,
        ArtifactDependency,
        KnownArtifact,
        registry::{
            ArtifactRegistry,
            ArtifactRegistryOperations,
            InMemoryArtifactRegistry
        }
    };
    use crate::util::util::Version;


    #[tokio::test]
    pub async fn test_install() -> Result<(), anyhow::Error> {

        let temp_dir = tempfile::tempdir()?;
        let movement_dir = MovementDir::new(&temp_dir.path().to_path_buf());
        movement_dir.store()?;

        let installer = MovementInstaller {
            basic_installer : BasicInstaller
        };

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

        let movement_dir = installer.install(
            movement_dir, 
            &registry,
            vec![
                ArtifactDependency::identifier(
                    KnownArtifact::Name("moons".to_string()),
                    Version::new(0, 0, 0)
                )
            ]
        ).await?;

        assert_eq!(movement_dir.requirements.0.len(), 1);
        assert_eq!(movement_dir.resolutions.0.len(), 2);

        // make sure we've stored the movement dir
        let loaded_movement_dir = movement_dir.clone().load()?;
        assert_eq!(loaded_movement_dir, movement_dir);

        Ok(())

    }

}
