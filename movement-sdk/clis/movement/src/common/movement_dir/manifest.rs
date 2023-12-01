use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use super::m1::M1Manifest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementDirManifest {
    pub movement_dir : Option<PathBuf>,
    pub movement_binary : Option<PathBuf>,
    pub m1 : Option<M1Manifest>
}

impl MovementDirManifest {

    pub fn new(movement_dir : Option<PathBuf>, movement_binary : Option<PathBuf>, m1 : Option<M1Manifest>) -> Self {
        Self {
            movement_dir,
            movement_binary,
            m1
        }
    }

    pub fn register_movement_dir_path(&mut self, movement_dir : PathBuf) {
        self.movement_dir = Some(movement_dir);
    }

    pub fn remove_movement_dir_path(&mut self) {
        self.movement_dir = None;
    }

    pub fn register_movement_binary_path(&mut self, movement_binary : PathBuf) {
        self.movement_binary = Some(movement_binary);
    }

    pub fn remove_movement_binary_path(&mut self) {
        self.movement_binary = None;
    }

}