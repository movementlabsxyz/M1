use std::collections::BTreeSet;
use super::ArtifactDependency;
use serde::{Serialize, Deserialize};


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactRequirements(pub BTreeSet<ArtifactDependency>);

impl ArtifactRequirements {

    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn remove(&mut self, dependency : &ArtifactDependency) {
        self.0.remove(dependency);
    }

    pub fn add(&mut self, dependency : ArtifactDependency) {
        self.0.insert(dependency);
    }

    pub fn add_all(&mut self, dependencies : &mut ArtifactRequirements) {
        self.0.append(&mut dependencies.0);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

}