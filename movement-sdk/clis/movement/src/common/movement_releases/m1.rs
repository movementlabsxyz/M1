use serde::{Serialize, Deserialize};
use super::{Release, movement_releases::MovementGitHubRelease};
use crate::common::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M1GitHubReleases {
    pub m1_source_with_submodules : Release,
    pub m1_subnet_binary : Release,
}

impl M1GitHubReleases {

    pub fn from_os_arch(version : &Version) -> Self {
       Self {
            m1_source_with_submodules : MovementGitHubRelease::new(
                "movemntdev".to_string(), 
                "M1".to_string(), 
                version.clone(), 
                "m1-source-with-submodules".to_string(),
                ".tar.gz".to_string()
            ).into(),
            m1_subnet_binary : MovementGitHubRelease::new(
                "movemntdev".to_string(), 
                "M1".to_string(), 
                version.clone(), 
                "subnet".to_string(),
                "".to_string()
            ).into()

       }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum M1Releases {
    GitHub(M1GitHubReleases)
}

impl M1Releases {

    pub fn from_os_arch(version : &Version) -> Self {
        Self::GitHub(M1GitHubReleases::from_os_arch(version))
    }

    pub fn m1_source_with_submodules(&self) -> &Release {
        match self {
            Self::GitHub(releases) => &releases.m1_source_with_submodules
        }
    }

    pub fn m1_subnet_binary(&self) -> &Release {
        match self {
            Self::GitHub(releases) => &releases.m1_subnet_binary
        }
    }

}

#[cfg(test)]
mod test {

    use super::*;

    // this is primarily for a manual check right now
    // run this and check the dir which is printed.
    #[tokio::test]
    async fn test_m1_github_releases() -> Result<(), anyhow::Error> {

        let m1_releases = M1Releases::from_os_arch(&Version::Latest);
        println!("{:?}", m1_releases);

        // tmp dir
        let tmp_dir = tempfile::tempdir().unwrap();
        println!("tmp_dir: {:?}", tmp_dir);

        // get all of the releases
        m1_releases.m1_subnet_binary().to_file(&tmp_dir.path().join("subnet")).await?;
        m1_releases.m1_source_with_submodules().to_file(&tmp_dir.path().join("m1-source-with-submodules.tar.gz")).await?;

        // check that they are there
        assert!(tmp_dir.path().join("subnet").exists());
        assert!(tmp_dir.path().join("m1-source-with-submodules.tar.gz").exists());
    
        Ok(())
        
    }

}