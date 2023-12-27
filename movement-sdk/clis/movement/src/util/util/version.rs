use semver::Version as SemVerVersion;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Version {
    Latest,
    Version(SemVerVersion),
}
