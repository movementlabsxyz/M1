use serde::{Serialize, Deserialize, ser::SerializeStruct};
use std::path::PathBuf;
use tempfile::TempDir as TempFileDir;
use std::hash::{Hash, Hasher};
use crate::util::util::with::WithMovement;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Location {
    Path(PathBuf),
    Noop,
    Unknown
}

impl From<PathBuf> for Location {
    fn from(path : PathBuf) -> Self {
        Self::Path(path)
    }
}
