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

impl VersionTolerance {

    pub fn permits(&self, left : &Version, right : &Version) -> bool {

        match self {
            VersionTolerance::Exact => {
                left == right
            },
            VersionTolerance::Compatible => {
                left == right
            },
            VersionTolerance::Greater => {
                left > right
            },
            VersionTolerance::GreaterOrEqual => {
                left >= right
            },
            VersionTolerance::Less => {
                left < right
            },
            VersionTolerance::LessOrEqual => {
                left <= right
            }
        }

    }

}