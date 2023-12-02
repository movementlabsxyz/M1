use serde::{Serialize, Deserialize};
use super::{Release, movement_releases::MovementGitHubRelease};
use crate::common::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementCliGithubReleases {
    pub movement_cli_binary : Release,
}

impl MovementCliGithubReleases {

    pub fn from_os_arch(version : &Version) -> Self {
       Self {
            movement_cli_binary : MovementGitHubRelease::new(
                "movemntdev".to_string(), 
                "M1".to_string(), 
                version.clone(), 
                "movement".to_string(),
                "".to_string()
            ).into()
       }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum M1Releases {
    GitHub(MovementCliGithubReleases)
}

impl M1Releases {

    pub fn from_os_arch(version : &Version) -> Self {
        Self::GitHub(MovementCliGithubReleases::from_os_arch(version))
    }

    pub fn movement_cli_binary(&self) -> &Release {
        match self {
            Self::GitHub(releases) => &releases.movement_cli_binary
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
        m1_releases.movement_cli_binary().to_file(&tmp_dir.path().join("movement")).await?;

        // check that they are there
        assert!(tmp_dir.path().join("movement").exists());
    
        Ok(())
        
    }

}