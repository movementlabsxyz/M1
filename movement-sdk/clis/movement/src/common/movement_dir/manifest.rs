use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use serde_json;
use super::m1::M1Manifest;
use super::super::movement_releases::Release;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestElement {
    pub release : Release,
    pub path : Option<PathBuf>
}

impl ManifestElement {

    pub fn new(release : Release, path : Option<PathBuf>) -> Self {
        Self {
            release,
            path
        }
    }

    pub fn register_path(&mut self, path : PathBuf) {
        self.path = Some(path);
    }

    pub fn remove_path(&mut self) {
        self.path = None;
    }

    pub fn release(&self) -> &Release {
        &self.release
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub async fn write(&self) -> Result<(), anyhow::Error> {
        match self.path {
            Some(ref path) => {
                self.release.to_file(path).await
            },
            None => {
                Err(anyhow::anyhow!("No path registered for this manifest element"))
            }
        }
    }

    pub async fn write_if_path_defined(&self) -> Result<(), anyhow::Error> {
        match self.path {
            Some(ref path) => {
                self.release.to_file(path).await
            },
            None => {
                Ok(())
            }
        }
    }

    // todo: update to include directories
    pub async fn remove(&self) -> Result<(), anyhow::Error> {
        match self.path {
            Some(ref path) => {
                fs::remove_file(path)?;
                Ok(())
            },
            None => {
                Err(anyhow::anyhow!("No path registered for this manifest element"))
            }
        }
    }

    // todo: update to include directories
    pub async fn remove_if_path_defined(&self) -> Result<(), anyhow::Error> {
        match self.path {
            Some(ref path) => {
                fs::remove_file(path)?;
                Ok(())
            },
            None => {
                Ok(())
            }
        }
    }

    pub async fn register_and_write(&mut self, path : PathBuf) -> Result<(), anyhow::Error> {
        self.register_path(path);
        self.write().await
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementDirManifest {
    pub movement_dir : ManifestElement,
    pub movement_binary : ManifestElement,
    pub m1 : M1Manifest
}

impl MovementDirManifest {

    pub fn new(movement_dir : ManifestElement, movement_binary : ManifestElement, m1 : M1Manifest) -> Self {
        Self {
            movement_dir,
            movement_binary,
            m1
        }
    }

    pub fn update_manifest_file(&self) -> Result<(), anyhow::Error> {

        match self.movement_dir.path() {
            Some(path) => {
                // Serialize `self` to a JSON string
                let serialized = serde_json::to_string(&self)?;

                // Construct the file path
                let file_path = path.join("manifest.json");

                // Write the serialized data to the file
                fs::write(file_path, serialized)?;

                Ok(())
            },
            None => {
                Err(anyhow::anyhow!("No path registered for this manifest element"))
            }
        }
        
    }

    pub async fn get_all_defined(&self) -> Result<(), anyhow::Error> {
        tokio::try_join!(
            self.movement_dir.write_if_path_defined(),
            self.movement_binary.write_if_path_defined(),
            self.m1.get_all_defined()
        )?;
        Ok(())
    }

}