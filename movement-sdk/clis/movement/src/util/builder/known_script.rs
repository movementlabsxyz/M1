use super::{Builder, BuilderOperations};
use crate::util::{
    artifact::Artifact,  
    release::ReleaseOperations
};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KnownScriptPart {
    pub command : String,
    pub args : Vec<String>,
    pub env : Vec<(String, String)>,
    pub working_directory : PathBuf
}

impl KnownScriptPart {
    
    pub fn new(
        command : String, 
        args : Vec<String>, 
        env : Vec<(String, String)>, 
        working_directory : PathBuf
    ) -> Self {
        Self {
            command,
            args,
            env,
            working_directory
        }
    }

    pub async fn exec(&self) -> Result<(), anyhow::Error> {

        let mut command = std::process::Command::new(&self.command);
        command.args(&self.args);
        for (key, value) in self.env.iter() {
            command.env(key, value);
        };
        command.current_dir(&self.working_directory);

        let status = command.status().expect("Failed to execute process.");

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to execute known script.")
        }

    }

}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KnownScript {
    pub build_command : KnownScriptPart,
    pub remove_command : KnownScriptPart
}

impl KnownScript {

    pub fn new(build_command : KnownScriptPart, remove_command : KnownScriptPart) -> Self {
        Self {
            build_command,
            remove_command
        }
    }

}

#[async_trait::async_trait]
impl BuilderOperations for KnownScript {

    async fn build(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        artifact.release.get(&artifact.location).await?;

        self.build_command.exec().await?;
        
        Ok(artifact.clone())

    }

    async fn remove(&self, artifact : &Artifact) -> Result<Artifact, anyhow::Error> {

        self.remove_command.exec().await?;

        Ok(artifact.clone())

    }

}

#[cfg(test)]
pub mod test {

    use super::*;
    use crate::util::{
        artifact::Artifact, 
        release::Release, 
        location::Location,  
        util::Version,
        builder::Builder,
        checker::Checker
    };
    use std::collections::BTreeSet;

    #[tokio::test]
    async fn test_known_script() -> Result<(), anyhow::Error> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().to_path_buf().join("hello.txt");

        let full_command = format!("echo hello > {}", path.to_str().unwrap());

        let known_script = KnownScript::new(
            KnownScriptPart::new(
                "sh".to_string(),
                vec!["-c".to_string(), full_command],
                vec![],
                PathBuf::from(".")
            ),
            KnownScriptPart::new(
                "rm".to_string(),
                vec![path.to_str().unwrap().to_string()],
                vec![],
                PathBuf::from(".")
            )
        );

        let artifact = Artifact::new(
            Release::Noop,
            Location::Unknown,
            Version::Latest,
            Builder::Noop,
            Checker::Noop,
            BTreeSet::new()
        );

        known_script.build(&artifact).await?;
        let contents = std::fs::read_to_string(&path)?;
        assert_eq!(contents, "hello\n");

        known_script.remove(&artifact).await?;
        assert!(!path.exists());

        Ok(())
    }

}

impl Into<Builder> for KnownScript {
    fn into(self) -> Builder {
        Builder::KnownScript(self)
    }
}