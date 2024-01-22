use super::{Builder, BuilderOperations};
use crate::util::{
    artifact::Artifact,  
    release::ReleaseOperations
};
use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use crate::movement_dir::MovementDir;

#[cfg(feature = "logging")]
use std::thread;

use std::io::Write;
use std::process::Stdio;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScriptPart {
    pub script : String,
    pub env : Vec<(String, String)>,
    pub working_directory : PathBuf
}

impl ScriptPart {
    
    pub fn new(
        script : String, 
        env : Vec<(String, String)>, 
        working_directory : PathBuf
    ) -> Self {
        Self {
            script,
            env,
            working_directory
        }
    }

    pub async fn exec(&self, movement : &MovementDir) -> Result<(), anyhow::Error> {

        // todo: switch to pseudo terminal to preserve colors
        let mut command = std::process::Command::new("bash");
    
        for (key, value) in self.env.iter() {
            command.env(key, value);
        };
        
        // add the movement context
        let movement_dir = match movement.path.to_str() {
            Some(movement_dir) => movement_dir,
            None => anyhow::bail!("Failed to convert movement path to string.")
        };
        command.env("MOVEMENT_DIR", movement_dir);
        let movement_manifest = match movement.manifest_path.to_str() {
            Some(movement_manifest) => movement_manifest,
            None => anyhow::bail!("Failed to convert movement manifest path to string.")
        };
        command.env("MOVEMENT_MANIFEST", movement_manifest);

        command.current_dir(&self.working_directory);
        
        // spawn the child
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let stdin = match child.stdin.as_mut() {
                Some(stdin) => stdin,
                None => anyhow::bail!("Failed to open stdin."),
            };
            stdin.write_all(self.script.as_bytes())?;
        }


        #[cfg(feature = "logging")]
        let (stdout_handle, stderr_handle) = {
            let mut stdout = child.stdout.take().expect("Failed to take stdout");
            let mut stderr = child.stderr.take().expect("Failed to take stderr");

            let stdout_handle = thread::spawn(move || {
                if let Err(e) = std::io::copy(&mut stdout, &mut std::io::stdout()) {
                    eprintln!("Error writing to stdout: {}", e);
                }
            });
            let stderr_handle = thread::spawn(move || {
                if let Err(e) = std::io::copy(&mut stderr, &mut std::io::stderr()) {
                    eprintln!("Error writing to stderr: {}", e);
                }
            });

            (stdout_handle, stderr_handle)
        };


        let status = child.wait().expect("Failed to wait on child");

        #[cfg(feature = "logging")]
        {
            stdout_handle.join().expect("Failed to join stdout thread");
            stderr_handle.join().expect("Failed to join stderr thread");
        }

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("Script execution failed.")
        }

    }

}

impl From<String> for ScriptPart {
    fn from(script : String) -> Self {
        Self {
            script,
            env : vec![],
            working_directory : PathBuf::from(".")
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Script {
    pub build_command : ScriptPart,
    pub remove_command : ScriptPart
}

impl From<String> for Script {
    fn from(script : String) -> Self {
        Self {
            build_command : ScriptPart::from(script.clone()),
            remove_command : ScriptPart::from("".to_string())
        }
    }
}

impl Script {

    pub fn new(build_command : ScriptPart, remove_command : ScriptPart) -> Self {
        Self {
            build_command,
            remove_command
        }
    }

}

#[async_trait::async_trait]
impl BuilderOperations for Script {

    async fn build(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        artifact.release.get(&artifact.location).await?;

        self.build_command.exec(movement).await?;
        
        Ok(artifact.clone())

    }

    async fn remove(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        self.remove_command.exec(movement).await?;

        Ok(artifact.clone())

    }

}

impl Into<Builder> for Script {
    fn into(self) -> Builder {
        Builder::Script(self)
    }
}

#[cfg(test)]
pub mod test {

    use super::*;
    use crate::util::{
        artifact::{Artifact, KnownArtifact}, 
        release::Release, 
        location::Location,  
        util::Version,
        builder::Builder,
        checker::Checker
    };
    use std::collections::BTreeSet;

    // ? Run with `cargo test --features "logging" test_known_script_not_relative -- --nocapture` to see logging
    #[tokio::test]
    async fn test_known_script_not_relative() -> Result<(), anyhow::Error> {

        let dir = tempfile::tempdir()?;
        let movement_dir = MovementDir::new(&dir.path().to_path_buf());

        let file_dir = tempfile::tempdir()?;
        let path = file_dir.path().to_path_buf().join("hello.txt");

        let build_script = format!(r#"
            echo $MOVEMENT_DIR
            echo hello > {}
        "#, path.to_str().unwrap());

        let remove_script = format!(r#"
            rm {}
        "#, path.to_str().unwrap());

        let known_script = Script::new(
            ScriptPart::new(
                build_script,
                vec![],
                PathBuf::from(".")
            ),
            ScriptPart::new(
                remove_script,
                vec![],
                PathBuf::from(".")
            )
        );

        let artifact = Artifact::new(
            KnownArtifact::Test,
            Release::Noop,
            Location::Unknown,
            Version::Latest,
            Builder::Noop,
            Checker::Noop,
            BTreeSet::new()
        );

        known_script.build(&artifact, &movement_dir).await?;
        let contents = std::fs::read_to_string(&path)?;
        assert_eq!(contents, "hello\n");

        known_script.remove(&artifact, &movement_dir).await?;
        assert!(!path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_script_uses_relative_properties() -> Result<(), anyhow::Error> {

        let dir = tempfile::tempdir()?;
        let movement_dir = MovementDir::new(&dir.path().to_path_buf());
        let path = movement_dir.path.to_path_buf().join("hello.txt");

        let build_script = format!(r#"
            echo $MOVEMENT_DIR
            echo $MOVEMENT_DIR > $MOVEMENT_DIR/hello.txt
        "#);

        let remove_script = format!(r#"
            rm $MOVEMENT_DIR/hello.txt
        "#);

        let known_script = Script::new(
            ScriptPart::new(
                build_script,
                vec![],
                PathBuf::from(".")
            ),
            ScriptPart::new(
                remove_script,
                vec![],
                PathBuf::from(".")
            )
        );

        let artifact = Artifact::new(
            KnownArtifact::Test,
            Release::Noop,
            Location::Path(movement_dir.path.to_path_buf()),
            Version::Latest,
            Builder::Noop,
            Checker::Noop,
            BTreeSet::new()
        );

        known_script.build(&artifact, &movement_dir).await?;
        let contents = std::fs::read_to_string(&path)?;
        assert_eq!(contents, format!("{}\n", &movement_dir.path.to_str().unwrap()));
        known_script.remove(&artifact, &movement_dir).await?;
        assert!(!path.exists());

        Ok(())
    }

}