use serde::{Serialize, Deserialize};
use std::path::{PathBuf, Path};
use super::m1::M1Releases;
use reqwest;
use std::io::Write;
use crate::common::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]

pub enum Release {
    HttpGET(String),
    File(PathBuf),
    Unknown
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                format!("https://github.com/{}/{}/releases/latest/download/{}-{}-{}{}", self.owner, self.repo, self.asset, std::env::consts::OS, std::env::consts::ARCH, self.suffix)
            },
            Version::Version(version) => {
                format!("https://github.com/{}/{}/releases/download/{}/{}-{}-{}{}", self.owner, self.repo, version, self.asset, std::env::consts::OS, std::env::consts::ARCH, self.suffix)
            }
        }
    }

}

impl Into<Release> for MovementGitHubRelease {
    fn into(self) -> Release {
        Release::HttpGET(self.os_arch_release_url())
    }
}

impl Release  {
    
    pub async fn to_file(&self, path : &Path) -> Result<(), anyhow::Error> {
        match self {
            Release::HttpGET(url) => {
                let mut response = reqwest::get(url).await?;
                if response.status().is_success() {
                    let mut file = std::fs::File::create(path)?;
                    while let Some(chunk) = response.chunk().await? {
                        file.write_all(&chunk)?;
                    }
                    Ok(())
                } else {
                    // Handle HTTP errors
                    Err(anyhow::format_err!("HTTP request failed with status: {}", response.status()))
                }
            },
            Release::File(to_path) => {
                // copy the release file to the path
                std::fs::copy(path, to_path)?;
                Ok(())
            },
            _ => {
                Err(anyhow::format_err!("Cannot get release to file for unknown release type."))
            }
        }
    }
    

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementReleases {
    m1_releases : M1Releases
}

#[cfg(test)]
mod test {

    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_get_release_to_file() -> Result<(), anyhow::Error> {

        let release_text = "hello";

        let release = Release::HttpGET(String::from("https://github.com/movemntdev/resources/releases/download/v0.0.0/hello.txt"));

        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        release.to_file(&path).await.unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();

        assert_eq!(contents, release_text);

        Ok(())

    }

    #[tokio::test]
    async fn test_fails_for_nonexistent_release() -> Result<(), anyhow::Error> {

        let release = Release::HttpGET(String::from("https://github.com/invalid/uri"));
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        release.to_file(&path).await.expect_err("Should fail for nonexistent release.");

        Ok(())

    }

}