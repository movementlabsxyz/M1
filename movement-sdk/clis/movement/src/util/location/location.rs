use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedFiles {
    pub release_target_paths : Vec<PathBuf>, // ! this needs to be string instead of PathBuf to accommodate complexity
    pub artifact_target_paths : Vec<PathBuf> // ! this needs to be string instead of PathBuf to accommodate complexity
}

impl StagedFiles {

    pub fn new(release_target_paths : Vec<PathBuf>, artifact_target_paths : Vec<PathBuf>) -> Self {
        Self {
            release_target_paths,
            artifact_target_paths
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryLocationBytes(Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Location {
    StagedFiles(StagedFiles),
    TryLocationBytes(TryLocationBytes),
    Unknown
}