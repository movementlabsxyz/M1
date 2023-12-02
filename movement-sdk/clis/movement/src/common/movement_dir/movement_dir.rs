use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use super::manifest::MovementDirManifest;

pub trait DefaultInMovementDir {
    fn default_in_movement_dir(path : &PathBuf) -> Self;
}

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

    pub fn manifest(&self) -> &MovementDirManifest {
        &self.manifest
    }

    pub fn update_manifest_file(&self) -> Result<(), anyhow::Error> {
        self.manifest.update_manifest_file()?;
        Ok(())
    }

    pub async fn get_all_defined(&self) -> Result<(), anyhow::Error> {
        self.manifest.get_all_defined().await?;
        Ok(())
    }

}