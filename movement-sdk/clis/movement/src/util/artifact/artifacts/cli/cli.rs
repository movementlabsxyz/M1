use crate::util::release::ReleaseOperations;
pub use crate::util::release::releases::cli::ReleaseBuilder;
use serde::{Deserialize, Serialize};
pub use crate::util::location::Location;
use crate::util::util::Version;
use crate::util::artifact::Artifact;
use crate::util::builder::Builder;
use crate::util::checker::Checker;
use std::collections::BTreeSet;
use crate::util::sys::{Arch, OS};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactBuilder;

impl ArtifactBuilder {

    pub fn default_artifact() -> Artifact {
        let home = dirs::home_dir().unwrap();
        let cli = home.join(".movement").join("movement");
        Artifact::new(
            ReleaseBuilder::m1_repo()
            .with_arch(&Arch::Aarch64)
            .with_os(&OS::Linux),
            Location::temp(
                "movement".to_string(),
                &cli
            ),
            Version::Latest,
            Builder::Unknown,
            Checker::Noop,
            BTreeSet::new()
        )
    }

}

