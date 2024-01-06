use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use serde_json;
use crate::util::artifact::{Artifact, ArtifactStatus};
use std::fs;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestElement {
    pub artifact : Artifact,
    pub status : ArtifactStatus
}

impl ManifestElement {
    
    pub fn new(artifact : Artifact, status : ArtifactStatus) -> Self {
        Self {
            artifact,
            status
        }
    }
    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementDirManifest {
    pub movement_dir : ManifestElement,
    pub movement_binary : ManifestElement,
}