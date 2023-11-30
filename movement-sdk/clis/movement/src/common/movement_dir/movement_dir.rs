use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M1Manifest {
    pub m1_source : Option<PathBuf>,
    pub subnet_binary : Option<PathBuf>,
    pub proxy_binary : Option<PathBuf>
}

impl M1Manifest {
    pub fn new(m1_source : Option<PathBuf>, subnet_binary : Option<PathBuf>, proxy_binary : Option<PathBuf>) -> Self {
        Self {
            m1_source,
            subnet_binary,
            proxy_binary
        }
    }
}

impl Default for M1Manifest {
    fn default() -> Self {
        Self {
            m1_source : None,
            subnet_binary : None,
            proxy_binary : None
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementDirManifest {
    pub movement_dir : PathBuf,
    pub movement_binary : PathBuf,
    pub m1 : Option<M1Manifest>
}

#[derive(Debug, Clone)]
pub struct MovementDir {
    pub path: PathBuf,
    pub manifest : MovementDirManifest
}

impl MovementDir {

    // basic operations
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn path_str(&self) -> &str {
        self.path.to_str().unwrap()
    }

    pub fn path_str_with_trailing_slash(&self) -> String {
        format!("{}/", self.path_str())
    }

    pub fn path_str_without_trailing_slash(&self) -> String {
        self.path_str().to_string()
    }

    // m1 assets
    pub fn write_m1_source(&self, m1_source : PathBuf) -> Result<(), std::io::Error> {
        let mut manifest = self.manifest.clone();
        manifest.m1 = Some(M1Manifest::new(Some(m1_source), None, None));
        self.write_manifest(manifest)
    }

}

impl Default for MovementDir {
    fn default() -> Self {
        Self {
            // ! default path is $HOME/.movement
            path : 
        }
    }
}