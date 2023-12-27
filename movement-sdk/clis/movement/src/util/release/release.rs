use serde::{Serialize, Deserialize};
use std::path::{PathBuf, Path};
use reqwest;
use std::io::Write;
use crate::util::util::Version;
use crate::util::location::Location;
use super::file_release::File;
use super::http_get_release::HttpGET;
use super::movement_github_platform_release::MovementGitHubPlatformRelease;
use super::movement_github_release::MovementGitHubRelease;

#[async_trait::async_trait]
pub trait ReleaseOperations {

    /// Gets a release to a particular location.
    async fn get(&self, location : &Location) -> Result<(), anyhow::Error>;

}

#[derive(Debug, Clone, Serialize, Deserialize)]

pub enum Release {
    HttpGET(HttpGET),
    File(File),
    MovementGitHubPlatformRelease(MovementGitHubPlatformRelease),
    MovementGitHubRelease(MovementGitHubRelease),
    Unknown
}

#[async_trait::async_trait]
impl ReleaseOperations for Release {

    async fn get(&self, location : &Location) -> Result<(), anyhow::Error> {

        match self {
            Release::HttpGET(get) => {
                get.get(location).await
            },
            Release::File(file) => {
                file.get(location).await
            },
            Release::MovementGitHubPlatformRelease(release) => {
                release.get(location).await
            },
            Release::MovementGitHubRelease(release) => {
                release.get(location).await
            },
            _ => {
                anyhow::bail!("Cannot get an unsupported release type.");
            }
        }

    }

}