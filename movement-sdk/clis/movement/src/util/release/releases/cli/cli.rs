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

    #[tokio::test]
    async fn test_with_version() -> Result<(), anyhow::Error> {

        // todo: right now not all architectures and os's are supported
        // todo: we're going to use linux and x86_64 for now
        // todo: in the future we should change this to just detect
        let cli_release = ReleaseBuilder::m1_repo()
        .with_arch(
            &Arch::X86_64,
        )
        .with_os(
            &OS::Linux,
        );
        let location = Location::temp_staged_single()?;

        cli_release.get(&location).await?;
    
        Ok(())

    }


}

