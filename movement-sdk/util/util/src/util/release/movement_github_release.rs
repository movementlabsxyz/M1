use serde::{Serialize, Deserialize};
use super::{ReleaseOperations, Release};
use super::http_get_release::HttpGET;
use crate::util::util::Version;
use crate::util::location::Location;
use crate::util::sys::{Arch, OS};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MovementGitHubRelease {
    pub owner : String,
    pub repo : String,
    pub version : Version,
    pub asset : String,
    pub suffix : String
}

impl MovementGitHubRelease {

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
                format!("https://github.com/{}/{}/releases/latest/download/{}{}", self.owner, self.repo, self.asset, self.suffix)
            },
            Version::Version(version) => {
                format!("https://github.com/{}/{}/releases/download/{}/{}{}", self.owner, self.repo, version, self.asset, self.suffix)
            }
        }
    }

}

#[async_trait::async_trait]
impl ReleaseOperations for MovementGitHubRelease {

    async fn get(&self, location : &Location) -> Result<Location, anyhow::Error> {

        let http_get = HttpGET::new(self.os_arch_release_url());
        http_get.get(location).await?;
        Ok(location.clone())

    }

    fn with_version(mut self, version : &Version) -> Self {
        self.version = version.clone();
        self
    }

    fn with_arch(mut self, arch : &Arch) -> Self {
        self
    }

    fn with_os(mut self, os : &OS) -> Self {
        self
    }

}

impl Into<Release> for MovementGitHubRelease {
    fn into(self) -> Release {
        Release::MovementGitHubRelease(self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use semver::Version as SemVerVersion;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_get_hello() -> Result<(), anyhow::Error> {

        let release_text = "hello";
        
        let dir = tempdir()?;
        let path = dir.path().join("test.txt");
        let release = MovementGitHubRelease::new(
            "movemntdev".to_string(),
            "resources".to_string(),
            Version::Version(SemVerVersion::new(0, 0, 0)),
            "hello".to_string(),
            ".txt".to_string()
        );
        release.get(&path.clone().into()).await?;

        let contents = std::fs::read_to_string(&path).unwrap();

        assert_eq!(contents, release_text);

        Ok(())

    }

}