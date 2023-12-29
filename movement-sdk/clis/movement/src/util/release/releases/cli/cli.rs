use crate::util::release::{
    Release,
    movement_github_platform_release::MovementGitHubPlatformRelease
};
use serde::{Serialize, Deserialize};
use crate::util::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseBuilder;

impl ReleaseBuilder {

    pub fn m1_repo() -> Release {
        let release = MovementGitHubPlatformRelease::new(
            "movemntdev".to_string(),
            "M1".to_string(),
            Version::Latest,
            "movement".to_string(),
            "".to_string()
        );
        release.into()
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

        // todo: right now not all architectures and os's are supported
        // todo: we're going to use linux and x86_64 for now
        // todo: in the future we should change this to just detect
        let dir = tempfile::tempdir()?;
        let cli_release = ReleaseBuilder::m1_repo()
        .with_arch(
            &Arch::X86_64,
        )
        .with_os(
            &OS::Linux,
        );
        let location = Location::temp(
            "test.txt".to_string(), 
            &PathBuf::from("test.txt")
        );

        cli_release.get(&location).await?;
    
        Ok(())

    }


}

