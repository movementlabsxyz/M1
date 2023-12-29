use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use tempfile;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ReleaseAndArtifact {
    pub release : PathBuf,
    pub artifact : PathBuf
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NamedFiles {
    pub release_files : BTreeMap<String, PathBuf>,
    pub artifact_files : BTreeMap<String, PathBuf>
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StagedFiles {
    pub release_target_paths : Vec<PathBuf>,
    pub artifact_target_paths : Vec<PathBuf>
}

impl StagedFiles {

    pub fn new(release_target_paths : Vec<PathBuf>, artifact_target_paths : Vec<PathBuf>) -> Self {
        Self {
            release_target_paths,
            artifact_target_paths
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TryLocationBytes(Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Location {
    ReleaseAndArtifact(ReleaseAndArtifact),
    NamedFiles(NamedFiles),
    StagedFiles(StagedFiles), // Prefer ReleaseAndArtifact or NamedFiles
    TryLocationBytes(TryLocationBytes),
    Unknown
}

impl Location {

    pub fn temp_staged_single() -> Result<Self, anyhow::Error> {
        let dir = tempfile::tempdir()?;
        let release_path = dir.path().to_path_buf().join("release");
        let artifact_path = dir.path().to_path_buf().join("artifact");
        Ok(Self::StagedFiles(StagedFiles::new(
            vec![release_path], 
            vec![artifact_path]
        )))
    }

}