use serde::{Serialize, Deserialize, ser::SerializeStruct};
use std::path::PathBuf;
use tempfile::TempDir as TempFileDir;
use std::hash::{Hash, Hasher};

/// Staged is a wrapper around an optional TempFileDir and two PathBuf instances.
/// It is intended to facilitate operations in a temporary directory before 
/// finalizing artifacts to a specified destination.
#[derive(Debug)]
pub struct Staged {
    pub temp_dir: Option<TempFileDir>,
    pub release_destination: PathBuf, 
    pub artifact_destination: PathBuf,
}

impl Staged {
    /// Creates a new Staged instance with specified release and artifact destinations.
    pub fn new(release_destination: PathBuf, artifact_destination: PathBuf) -> Self {
        Staged {
            temp_dir: None,
            release_destination,
            artifact_destination,
        }
    }

    /// Sets a TempFileDir for the Staged instance.
    pub fn with_temp_dir(mut self, temp_dir: TempFileDir) -> Self {
        self.temp_dir = Some(temp_dir);
        self
    }

    // Add any other methods that are necessary for your application logic
}

impl Clone for Staged {
    fn clone(&self) -> Self {
        match &self.temp_dir {
            Some(temp_dir) => {
                Staged {
                    temp_dir: Some(tempfile::tempdir().expect("Failed to create temp dir")),
                    release_destination: self.release_destination.clone(),
                    artifact_destination: self.artifact_destination.clone(),
                }
            },
            None => {
                Staged {
                    temp_dir: None,
                    release_destination: self.release_destination.clone(),
                    artifact_destination: self.artifact_destination.clone(),
                }
            }
            
        }
    }
}

// Implement Serialize and Deserialize for Staged
impl Serialize for Staged {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        let mut state = serializer.serialize_struct("Staged", 2)?;
        state.serialize_field("release_destination", &self.release_destination)?;
        state.serialize_field("artifact_destination", &self.artifact_destination)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Staged {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de> {
        let (release_destination, artifact_destination) = <(PathBuf, PathBuf)>::deserialize(deserializer)?;
        Ok(Staged {
            temp_dir: None,
            release_destination,
            artifact_destination,
        })
    }
}

// Implement PartialEq, Eq, Hash, PartialOrd, Ord for Staged
impl PartialEq for Staged {
    fn eq(&self, other: &Self) -> bool {
        self.release_destination == other.release_destination &&
        self.artifact_destination == other.artifact_destination
    }
}

impl Eq for Staged {}

impl Hash for Staged {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.release_destination.hash(state);
        self.artifact_destination.hash(state);
    }
}

impl PartialOrd for Staged {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.artifact_destination.cmp(&other.artifact_destination))
    }
}

impl Ord for Staged {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.artifact_destination.cmp(&other.artifact_destination)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Location {
    Staged(Staged),
    Unknown
}

impl Location {

    pub fn staged(release_destination: PathBuf, artifact_destination: PathBuf) -> Self {
        Location::Staged(Staged::new(release_destination, artifact_destination))
    }

    pub fn temp(relative_release_destination : PathBuf, artifact_destination: PathBuf) -> Self {
        let temp = TempFileDir::new().expect("Failed to create temp dir");
        let release_destination = temp.path().join(relative_release_destination);
        Location::Staged(Staged {
            temp_dir: Some(temp),
            release_destination: release_destination,
            artifact_destination
        })
    }

    pub fn with_temp_dir(mut self, temp_dir: TempFileDir) -> Self {
        match &mut self {
            Location::Staged(staged) => {
                staged.temp_dir = Some(temp_dir);
            },
            _ => {}
        }
        self
    }

    // Implement other constructors as necessary
}

// Implement any additional logic, traits or functions as required
