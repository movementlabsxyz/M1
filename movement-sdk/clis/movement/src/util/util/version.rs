use semver::Version as SemVerVersion;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Version {
    Latest,
    Version(SemVerVersion),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VersionTolerance {
    Exact,
    Compatible,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}