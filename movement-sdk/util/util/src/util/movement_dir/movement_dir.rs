use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::util::artifact::{
    registry::ArtifactRegistry,
    requirements::ArtifactRequirements,
    resolution::ArtifactDependencyResolutions
};

pub trait DefaultInMovementDir {
    fn default_in_movement_dir(path : &PathBuf) -> Self;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MovementDir {
    pub path : PathBuf,
    pub manifest_path : PathBuf,
    pub requirements : ArtifactRequirements,
    pub resolutions : ArtifactDependencyResolutions,
}

impl MovementDir {

    const MOVEMENT_DIR_NAME : &'static str = ".movement";
    const MANIFEST_FILE_NAME : &'static str = "movement.ron";

    pub fn try_default_dir() -> Result<PathBuf, anyhow::Error> {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home dir"))?;
        Ok(home_dir.join(Self::MOVEMENT_DIR_NAME))
    }

    fn manifest_path(path : &PathBuf) -> PathBuf {
        path.join(Self::MANIFEST_FILE_NAME)
    }

    pub fn new(path : &PathBuf) -> Self {
        Self {
            path : path.clone(),
            manifest_path : Self::manifest_path(path),
            requirements : ArtifactRequirements::new(),
            resolutions : ArtifactDependencyResolutions::new(),
        }
    }

    pub fn from_file(path : &PathBuf) -> Result<Self, anyhow::Error> {

        // get the manifest path
        let manifest_path = Self::manifest_path(path);

        // check the manifest path exists
        if !manifest_path.exists() {
            anyhow::bail!("Movement dir manifest does not exist: {:?}", manifest_path);
        }

        // check the manifest path is a file
        if !manifest_path.is_file() {
            anyhow::bail!("Movement dir manifest is not a file: {:?}", manifest_path);
        }

        // read the manifest file contents
        let manifest_contents = std::fs::read_to_string(&manifest_path)?;

        // deserialize the manifest file contents using toml
        let movement_dir : Self = ron::from_str(&manifest_contents)?;

        Ok(movement_dir)

    }

    pub fn sync(self) -> Result<Self, anyhow::Error> {

       // if the path buf exists
       if self.path.try_exists()? 
        && self.manifest_path.try_exists()? { // ! time of check error possible

            #[cfg(feature = "logging")]
            println!("Loading movement dir: {:?}", self.path);

            self.load()
       } else {

            #[cfg(feature = "logging")]
            println!("Creating movement dir: {:?}", self.path);

            self.store()?;
            Ok(self)
       }

    }

    pub fn load(self) -> Result<Self, anyhow::Error> {

        Self::from_file(&self.path)

    }

    pub fn store(&self) -> Result<(), anyhow::Error> {

        // mkdir the parent
        std::fs::create_dir_all(&self.path)?;

        let manifest_contents = ron::to_string(&self)?;

        std::fs::write(&self.manifest_path, manifest_contents)?;

        Ok(())

    }

}

impl Default for MovementDir {

    fn default() -> Self {

        let movement_dir_path = Self::try_default_dir().expect("Could not find default movement dir");
        Self::new(&movement_dir_path)

    }

}

#[cfg(test)]
pub mod test {


    use super::*;
    use crate::util::artifact::{
        Artifact, 
        ArtifactDependency, 
        KnownArtifact,
        registry::ArtifactRegistryOperations
    };
    use crate::util::util::Version;

    #[tokio::test]
    pub async fn test_movement_dir() -> Result<(), anyhow::Error> {

        let temp_dir = tempfile::tempdir()?;
        let movement_dir = MovementDir::new(&temp_dir.path().to_path_buf());

        movement_dir.store()?;

        let outer_load_movement_dir = MovementDir::from_file(&temp_dir.path().to_path_buf())?;

        assert_eq!(outer_load_movement_dir, movement_dir);

        let old_movement_dir = movement_dir.clone();

        let loaded_movement_dir = movement_dir.load()?;

        assert_eq!(loaded_movement_dir, old_movement_dir);

        Ok(())

    }

    #[tokio::test]
    pub async fn test_movement_dir_with_artifacts() -> Result<(), anyhow::Error> {

        let temp_dir = tempfile::tempdir()?;
        let mut movement_dir = MovementDir::new(&temp_dir.path().to_path_buf());

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

        movement_dir.resolutions.add(
            moons_v0.clone().into(),
            moons_v0.clone()
        );

        movement_dir.requirements.add(moons_v0.clone().into());

        movement_dir.store()?;

        /*let old_movement_dir = movement_dir.clone();

        let loaded_movement_dir = movement_dir.load()?;

        assert_eq!(loaded_movement_dir, old_movement_dir);

        println!("loaded_movement_dir: {:?}", loaded_movement_dir);
        let dep : ArtifactDependency =  moons_v0.clone().into();
        assert!(loaded_movement_dir.requirements.0.contains(
            &dep
        ));*/

        Ok(())

    }

}