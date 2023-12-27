use serde::{Serialize, Deserialize};
use super::{ReleaseOperations, Release};
use super::http_get_release::HttpGET;
use crate::util::util::Version;
use crate::util::location::{
    Location,
    StagedFiles
};
use semver::Version as SemVerVersion;
use tempfile::tempdir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementGitHubPlatformRelease {
    pub owner : String,
    pub repo : String,
    pub version : Version,
    pub asset : String,
    pub suffix : String
}

impl MovementGitHubPlatformRelease {

    pub fn new(owner : String, repo : String, version : Version, asset : String, suffix : String) -> Self {
        Self {
            owner,
            repo,
            version,
            asset,
            suffix
        }
    }

    pub fn os_arch_release_url(&self) -> String {
        match &self.version {
            Version::Latest => {
                format!("https://github.com/{}/{}/releases/latest/download/{}-{}-{}{}", self.owner, self.repo, self.asset, std::env::consts::ARCH, std::env::consts::OS, self.suffix)
            },
            Version::Version(version) => {
                format!("https://github.com/{}/{}/releases/download/{}/{}-{}-{}{}", self.owner, self.repo, version, self.asset, std::env::consts::ARCH, std::env::consts::OS, self.suffix)
            }
        }
    }

}

#[async_trait::async_trait]
impl ReleaseOperations for MovementGitHubPlatformRelease {

    async fn get(&self, location : &Location) -> Result<(), anyhow::Error> {

        let http_get = HttpGET::new(self.os_arch_release_url());
        http_get.get(location).await

    }

}

impl Into<Release> for MovementGitHubPlatformRelease {
    fn into(self) -> Release {
        Release::MovementGitHubPlatformRelease(self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_get_hello() -> Result<(), anyhow::Error> {

            let release_text = "hello";
            
            let dir = tempdir().unwrap();
            let path = dir.path().join("test.txt");
            let location = Location::StagedFiles(
                StagedFiles::new(
                    vec![path.clone()],
                    vec![]
                )
            );
            let release = MovementGitHubPlatformRelease::new(
                "movemntdev".to_string(),
                "resources".to_string(),
                Version::Version(SemVerVersion::new(0, 0, 0)),
                "hello".to_string(),
                ".txt".to_string()
            );
            release.get(&location).await?;

            let contents = std::fs::read_to_string(&path).unwrap();

            assert_eq!(contents, release_text);
    
            Ok(())
    }

   #[tokio::test]
   async fn test_get_hello_zip() -> Result<(), anyhow::Error> {

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let location = Location::StagedFiles(
            StagedFiles::new(
                vec![path],
                vec![]
            )
        );
        let release = MovementGitHubPlatformRelease::new(
            "movemntdev".to_string(),
            "resources".to_string(),
            Version::Version(SemVerVersion::new(0, 0, 0)),
            "hello".to_string(),
            ".zip".to_string()
        );
        release.get(&location).await?;

        Ok(())

   }

}