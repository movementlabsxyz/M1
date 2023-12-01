use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M1Manifest {
    pub m1_source : Option<PathBuf>,
    pub m1_subnet_binary : Option<PathBuf>,
    pub m1_proxy_binary : Option<PathBuf>
}

impl M1Manifest {

    pub fn new(m1_source : Option<PathBuf>, m1_subnet_binary : Option<PathBuf>, m1_proxy_binary : Option<PathBuf>) -> Self {
        Self {
            m1_source,
            m1_subnet_binary,
            m1_proxy_binary
        }
    }

    pub fn register_m1_source_path(&mut self, m1_source : PathBuf) {
        self.m1_source = Some(m1_source);
    }

    pub fn remove_m1_source_path(&mut self) {
        self.m1_source = None;
    }

    pub fn register_m1_subnet_binary_path(&mut self, subnet_binary : PathBuf) {
        self.m1_subnet_binary = Some(subnet_binary);
    }

    pub fn remove_m1_subnet_binary_path(&mut self) {
        self.m1_subnet_binary = None;
    }

    pub fn register_m1_proxy_binary_path(&mut self, proxy_binary : PathBuf) {
        self.m1_proxy_binary = Some(proxy_binary);
    }

    pub fn remove_m1_proxy_path(&mut self) {
        self.m1_proxy_binary = None;
    }

}

impl Default for M1Manifest {
    fn default() -> Self {
        Self { 
            m1_source: None, 
            m1_subnet_binary: None, 
            m1_proxy_binary: None 
        }
    }
}