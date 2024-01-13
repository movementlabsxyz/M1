use serde::{Serialize, Deserialize};
use crate::util::util::Version;
use crate::util::location::Location;
use super::file_release::File;
use super::http_get_release::HttpGET;
use super::movement_github_platform_release::MovementGitHubPlatformRelease;
use super::movement_github_release::MovementGitHubRelease;
use crate::util::sys::{Arch, OS};

#[async_trait::async_trait]
pub trait ReleaseOperations {

    /// Gets a release to a particular location.
    async fn get(&self, location : &Location) -> Result<Location, anyhow::Error>;

    /// Sets the version for a release
    fn with_version(self, version : &Version) -> Self;

    /// Sets the arch for the release
    fn with_arch(self, arch : &Arch) -> Self;

    /// Sets the os for the release
    fn with_os(self, os : &OS) -> Self;

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]

pub enum Release {
    HttpGET(HttpGET),
    File(File),
    MovementGitHubPlatformRelease(MovementGitHubPlatformRelease),
    MovementGitHubRelease(MovementGitHubRelease),
    Noop,
    Unknown
}

impl Release {

    pub fn new() -> Self {
        Self::Unknown
    }

    pub fn github_platform_release(
        owner : String,
        repo : String,
        asset_name : String,
        suffix : String
    ) -> Self {
        Self::MovementGitHubPlatformRelease(
            MovementGitHubPlatformRelease::new(
                owner,
                repo,
                Version::Latest,
                asset_name,
                suffix
            )
        )
    }

}

#[async_trait::async_trait]
impl ReleaseOperations for Release {

    async fn get(&self, location : &Location) -> Result<Location, anyhow::Error> {

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
            Release::Noop => {
                Ok(location.clone())
            },
            _ => {
                anyhow::bail!("Cannot get an unsupported release type.");
            }
        }

    }

    fn with_version(self, version : &Version) -> Self {

       match self {
            Release::HttpGET(get) => {
                get.with_version(version).into()
            },
            Release::File(file) => {
                file.with_version(version).into()
            },
            Release::MovementGitHubPlatformRelease(release) => {
                release.with_version(version).into()
            },
            Release::MovementGitHubRelease(release) => {
                release.with_version(version).into()
            },
            Release::Noop => {
                self
            },
            _ => {
                self
            }
        }

    }

    fn with_arch(self, arch : &Arch) -> Self {

        match self {
            Release::HttpGET(get) => {
                get.with_arch(arch).into()
            },
            Release::File(file) => {
                file.with_arch(arch).into()
            },
            Release::MovementGitHubPlatformRelease(release) => {
                release.with_arch(arch).into()
            },
            Release::MovementGitHubRelease(release) => {
                release.with_arch(arch).into()
            },
            Release::Noop => {
                self
            },
            _ => {
               self
            }
        }

    }

    fn with_os(self, os : &OS) -> Self {

        match self {
            Release::HttpGET(get) => {
                get.with_os(os).into()
            },
            Release::File(file) => {
                file.with_os(os).into()
            },
            Release::MovementGitHubPlatformRelease(release) => {
                release.with_os(os).into()
            },
            Release::MovementGitHubRelease(release) => {
                release.with_os(os).into()
            },
            Release::Noop => {
                self
            },
            _ => {
                self
            }
        }

    }

}