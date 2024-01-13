use util::{
    release::Release,
    artifact::Artifact,
    util::util::patterns::constructor::ConstructorOperations
};
use crate::known_artifacts::third_party::sys::{curl, brew};

#[derive(Debug, Clone)]
pub struct Config;

#[derive(Debug, Clone)]
pub struct Constructor;

impl ConstructorOperations for Constructor {

    type Artifact = Artifact;
    type Config = Config;

    fn default() -> Self::Artifact {

        #[cfg(not(target_os = "windows"))]
        let movement = Artifact::bin_release(
            "movement".to_string(),
            Release::github_platform_release(
                "movemntlabs".to_string(),
                "m1".to_string(),
                "movement".to_string(),
                "".to_string()
            )
        );

        #[cfg(target_os = "windows")]
        let movement = Artifact::unsupported();

        movement

    }

    fn default_with_version(_ : &util::util::util::Version) -> Self::Artifact {
        Self::default()
    }

    fn from_config(_ : &Self::Config) -> Self::Artifact {
        Self::default()
    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use util::movement_dir::MovementDir;

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_movement() -> Result<(), anyhow::Error> {
        
        let temp_home = tempfile::tempdir()?;

        // Add any other essential system paths. This example includes /usr/bin and /bin.
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(),
        ])?;
        std::env::set_var("HOME", temp_home.path());
    
        // Perform test actions here
        let dir = temp_home.path().to_path_buf();
        let movement_dir = MovementDir::new(&dir);
        let artifact = Constructor::default();

        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), "/opt/homebrew/bin".to_string()
        ])?;
        artifact.install(&movement_dir).await?;

        // add /usr/local/bin back to path
        test_helpers::clean_path(vec![
            "/usr/bin".to_string(), "/bin".to_string(), "/usr/local/bin".to_string(), 
            dir.join("bin").to_str().unwrap().to_string()
        ])?;

        let exists = match std::process::Command::new("movement").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };

        assert!(exists);

        Ok(())

    }

    #[derive(Debug, Clone)]
    pub struct Fake;

    impl ConstructorOperations for Fake {

        type Artifact = Artifact;
        type Config = Config;

        fn default() -> Self::Artifact {
            Artifact::self_contained_script(
                "avalanche".to_string(),
                r#"
                    echo fake
                "#.to_string(),
            )
        }

        fn default_with_version(_ : &util::util::util::Version) -> Self::Artifact {
            Self::default()
        }

        fn from_config(_ : &Self::Config) -> Self::Artifact {
            Self::default()
        }

    }


}