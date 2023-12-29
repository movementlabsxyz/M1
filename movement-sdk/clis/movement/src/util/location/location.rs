use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use tempfile::TempDir as TempFileDir;
use std::fmt;
use std::hash::{Hash, Hasher};

/// TempDir is a wrapper around a TempFileDir and a PathBuf.
/// It is intended to accommodate cases wherein you wish to copy a file to a temporary directory
/// and perform some operations before shipping as an artifact. 
#[derive(Debug)]
pub struct TempDir {
    pub temp_dir: TempFileDir,
    pub release_tempfile_name: String, 
    pub artifact_file_destination: PathBuf,
}

impl TempDir {

    pub fn get_release_tempfile_path(&self) -> PathBuf {
        let mut path = self.temp_dir.path().to_path_buf();
        path.push(&self.release_tempfile_name);
        path
    }

}

// Implement Clone
impl Clone for TempDir {
    fn clone(&self) -> Self {
        Self {
            temp_dir: TempFileDir::new().expect("Failed to create temp dir"),
            release_tempfile_name: self.release_tempfile_name.clone(),
            artifact_file_destination: self.artifact_file_destination.clone(),
        }
    }
}

// Implement Serialize and Deserialize manually
impl Serialize for TempDir {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        // Serialize only the artifact_file_destination
        self.artifact_file_destination.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TempDir {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let artifact_file_destination = PathBuf::deserialize(deserializer)?;
        Ok(TempDir {
            temp_dir: TempFileDir::new().expect("Failed to create temp dir"),
            release_tempfile_name: String::new(),
            artifact_file_destination,
        })
    }
}

// Implement PartialEq, Eq, Hash, PartialOrd, Ord based on artifact_file_destination
impl PartialEq for TempDir {
    fn eq(&self, other: &Self) -> bool {
        self.artifact_file_destination == other.artifact_file_destination
    }
}

impl Eq for TempDir {}

impl Hash for TempDir {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.artifact_file_destination.hash(state);
    }
}

impl PartialOrd for TempDir {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.artifact_file_destination.partial_cmp(&other.artifact_file_destination)
    }
}

impl Ord for TempDir {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.artifact_file_destination.cmp(&other.artifact_file_destination)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Location {
    TempDir(TempDir),
    Unknown
}

impl Location {
    pub fn temp(name : String, known: &PathBuf) -> Self {
        let temp = TempFileDir::new().expect("Failed to create temp dir");
        Location::TempDir(TempDir {
            temp_dir: temp,
            release_tempfile_name: name, // Assuming an initial empty string
            artifact_file_destination: known.clone(),
        })
    }
}
