use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use super::manifest::MovementDirManifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementDir {
    pub manifest : MovementDirManifest
}

impl MovementDir {

    // basic operations
    pub fn new(manifest : MovementDirManifest) -> Self {
        Self {
            manifest
        }
    }


}