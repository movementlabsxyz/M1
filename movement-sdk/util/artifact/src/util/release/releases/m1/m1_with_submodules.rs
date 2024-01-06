use crate::util::release::{
    Release,
    movement_github_release::MovementGitHubRelease
};
use serde::{Serialize, Deserialize};
use crate::util::util::{
    Version,
    constructor::ConstructorOperations
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Release;
    type Config = Config;

    fn default_with_version(version : &Version) -> Self::Artifact {
       Self::sub_default_with_version::<M1Repo>(version)
    }

    fn from_config(config : &Self::Config) -> Self::Artifact {
        Self::sub_from_config::<M1Repo>(config)
    }

}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M1Repo;

impl ConstructorOperations for M1Repo {
    
    type Artifact = Release;
    type Config = Config;

    fn default_with_version(version : &Version) -> Self::Artifact {
        let release = MovementGitHubRelease::new(
            "movemntdev".to_string(),
            "M1".to_string(),
            version.clone(),
            "m1-with-submodules".to_string(),
            ".tar.gz".to_string()
        );
        release.into()
    }

    fn from_config(config : &Self::Config) -> Self::Artifact {
        Self::default()
    }

}



#[cfg(test)]
pub mod test {
    
    use super::*;
    use crate::util::{
        util::Version, 
        release::ReleaseOperations, 
        location::Location,
        sys::{Arch, OS}
    };

    use std::path::PathBuf;
    #[tokio::test]
    async fn test_latest() -> Result<(), anyhow::Error> {

        let cli_release = Constructor::default();
        let location = Location::temp(
            PathBuf::from("m1.tar.gz"), 
            PathBuf::from("m1.tar.gz")
        );

        cli_release.get(&location).await?;
    
        Ok(())

    }


}
