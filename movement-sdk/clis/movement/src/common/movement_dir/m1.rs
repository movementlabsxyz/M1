use std::path::PathBuf;
use serde::{Serialize, Deserialize};
use super::manifest::ManifestElement;
use crate::common::movement_releases::m1::M1Releases;
use crate::common::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M1Manifest {
    pub m1_source_with_submodules : ManifestElement,
    pub m1_subnet_binary : ManifestElement,
}

impl M1Manifest {

    pub fn new(m1_source_with_submodules : ManifestElement, m1_subnet_binary : ManifestElement, m1_proxy_binary : ManifestElement) -> Self {
        Self {
            m1_source_with_submodules,
            m1_subnet_binary,
        }
    }

    pub async fn get_all_defined(&self) -> Result<(), anyhow::Error> {
        tokio::try_join!(
            self.m1_source_with_submodules.write_if_path_defined(),
            self.m1_subnet_binary.write_if_path_defined()
        )?;
        Ok(())
    }

}

impl Default for M1Manifest {
    fn default() -> Self {

        let m1_releases = M1Releases::from_os_arch(&Version::Latest);

        Self { 
            
            m1_source_with_submodules : ManifestElement::new(
                m1_releases.m1_source_with_submodules().clone(),
                None
            ),
            m1_subnet_binary : ManifestElement::new(
                m1_releases.m1_subnet_binary().clone(),
                None
            )

        }
    }
}