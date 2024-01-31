use std::collections::BTreeSet;

use semver::Version as SemVerVersion;
use serde::{Serialize, Deserialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Version {
    Latest,
    Version(SemVerVersion),
}

impl Version {

    pub fn new(major : u64, minor : u64, patch : u64) -> Self {
        Self::Version(SemVerVersion::new(major, minor, patch))
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Version::Latest => write!(f, "latest"),
            Version::Version(version) => write!(f, "{}", version)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VersionTolerance {
    Exact,
    Minor,
    Major,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
}

impl Default for VersionTolerance {
    fn default() -> Self {
        VersionTolerance::Minor
    }
}

impl VersionTolerance {

    pub fn permits(&self, left : &Version, right : &Version) -> bool {

        match self {
            VersionTolerance::Exact => {
                left == right
            },
            VersionTolerance::Major => {
                match (left, right) {
                    (Version::Version(left), Version::Version(right)) => {
                        left.major == right.major
                    },
                    _ => left == right
                }
            },
            VersionTolerance::Minor => {
                match (left, right) {
                    (Version::Version(left), Version::Version(right)) => {
                        left.major == right.major && left.minor == right.minor
                    },
                    _ => left == right
                }
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VersionRange {
    pub left : Version,
    pub right : Version,
}

pub trait VersionManipulation {
    fn set_version(&mut self, version : Version);
    fn version(&self) -> Version;
}

pub trait EnumerateVersions {
    fn enumerate(&self, range : &VersionRange) -> BTreeSet<Self>
    where Self : Sized;
}

impl <T> EnumerateVersions for T where T : VersionManipulation + Clone + Ord {

    fn enumerate(&self, range : &VersionRange) -> BTreeSet<Self> {

        let mut enumeration = BTreeSet::new();

        let versions = range.enumerate();
        for version in versions {
            let mut artifact = self.clone();
            artifact.set_version(version);
            enumeration.insert(artifact);
        }

        enumeration

    }

}

impl VersionRange {

    pub fn new(left : Version, right : Version) -> Self {
        Self {
            left,
            right
        }
    }

    fn enumerate(&self) -> BTreeSet<Version> {

        let mut versions = BTreeSet::new();

        let mut current = self.left.clone();

        // todo: dangerous, could cause infinite loop
        while current <= self.right {
            versions.insert(current.clone());
            current = match current {
                Version::Version(version) => {
                    Version::Version(SemVerVersion::new(version.major, version.minor, version.patch + 1))
                },
                Version::Latest => {
                    Version::Latest
                }
            }
        }

        versions

    }

}

#[cfg(test)]
pub mod test {

    use super::*;

    #[test]
    pub fn test_permits_exact() -> Result<(), anyhow::Error> {

        let left_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);
        let right_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);

        assert!(VersionTolerance::Exact.permits(&left_v1_0_0, &right_v1_0_0));

        let v1_1_1 = Version::Version(SemVerVersion::parse("1.1.1")?);

        assert!(!VersionTolerance::Exact.permits(&left_v1_0_0, &v1_1_1));

        let latest = Version::Latest;

        assert!(!VersionTolerance::Exact.permits(&left_v1_0_0, &latest));
        assert!(!VersionTolerance::Exact.permits(&latest, &left_v1_0_0));
        assert!(!VersionTolerance::Exact.permits(&latest, &v1_1_1));
        assert!(VersionTolerance::Exact.permits(&latest, &latest));

        Ok(())

    }

    #[test]
    pub fn test_permits_gte() -> Result<(), anyhow::Error> {

        let left_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);
        let right_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);

        assert!(VersionTolerance::GreaterOrEqual.permits(&left_v1_0_0, &right_v1_0_0));

        let v1_1_1 = Version::Version(SemVerVersion::parse("1.1.1")?);

        assert!(VersionTolerance::GreaterOrEqual.permits(&v1_1_1, &left_v1_0_0));

        Ok(())

    }

    #[test]
    pub fn test_permits_major() -> Result<(), anyhow::Error> {

        let left_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);
        let right_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);

        assert!(VersionTolerance::Major.permits(&left_v1_0_0, &right_v1_0_0));

        let v1_1_1 = Version::Version(SemVerVersion::parse("1.1.1")?);

        assert!(VersionTolerance::Major.permits(&v1_1_1, &left_v1_0_0));

        Ok(())

    }

    #[test]
    pub fn test_permits_minor() -> Result<(), anyhow::Error> {

        let left_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);
        let right_v1_0_0 = Version::Version(SemVerVersion::parse("1.0.0")?);

        assert!(VersionTolerance::Minor.permits(&left_v1_0_0, &right_v1_0_0));

        let v1_1_1 = Version::Version(SemVerVersion::parse("1.1.1")?);

        assert!(!VersionTolerance::Minor.permits(&v1_1_1, &left_v1_0_0));

        Ok(())

    }

}