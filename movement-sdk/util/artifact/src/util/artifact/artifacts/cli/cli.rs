use crate::util::release::ReleaseOperations;
pub use crate::util::release::releases::cli as cli_release;
use serde::{Deserialize, Serialize};
pub use crate::util::location::Location;
use crate::util::util::{
    Version,
    patterns::constructor::ConstructorOperations
};
use crate::util::artifact::{Artifact, KnownArtifact};
use crate::util::builder::{Builder, download_release::DownloadRelease};
use crate::util::checker::Checker;
use std::collections::BTreeSet;
use crate::util::sys::{Arch, OS};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub path : PathBuf
}

impl Config {

    pub fn new(
        path : PathBuf
    ) -> Self {
        Self {
            path
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default_with_version(version : &Version) -> Self::Artifact {
        Self::sub_default_with_version::<Movement>(version)
    }

    fn from_config(config : &Self::Config) -> Self::Artifact {
        Self::sub_from_config::<Movement>(config)
    }

}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movement;

impl ConstructorOperations for Movement {

    type Artifact = Artifact;
    type Config = Config;

    fn default_with_version(version : &Version) -> Self::Artifact {
        let home = dirs::home_dir().unwrap();
        let cli = home.join(".movement").join("movement");
        Self::from_config(
            &Config::new(
                cli
            )
        )
    }

    fn from_config(config : &Self::Config) -> Self::Artifact {
        Artifact::new(
            KnownArtifact::Movement,
            cli_release::Constructor::default()
            // todo: revert to system detection
            .with_arch(&Arch::Aarch64)
            .with_os(&OS::Linux),
            // todo: add a Location::Path for simple cases like this
            // todo: it may also be that we want to have the builder pass unstaged locations to the release
            Location::staged(
                config.path.clone(),
                config.path.clone()
            ),
            Version::Latest,
            Builder::DownloadRelease(DownloadRelease::new()),
            Checker::Noop,
            BTreeSet::new()
        )
    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use std::path::Path;

    #[tokio::test]
    async fn test_default_artifact_builder() -> Result<(), anyhow::Error>{

        let home = tempfile::tempdir()?.into_path();
        let cli = home.join("movement");
        let config = Config::new(
            cli.clone()
        );

        let artifact = Constructor::from_config(&config);

        artifact.install().await?;

        assert!(
            Path::exists(
                cli.as_path()
            )
        );

        Ok(())

    }

}

