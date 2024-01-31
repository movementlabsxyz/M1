use serde::{Serialize, Deserialize};
use super::{ArtifactDependency, Artifact};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactDependencyResolutions(pub BTreeMap<ArtifactDependency, Artifact>);

impl ArtifactDependencyResolutions {

    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn add(&mut self, dependency : ArtifactDependency, artifact : Artifact) {
        self.0.insert(dependency, artifact);
    }

    pub fn remove(&mut self, dependency : &ArtifactDependency) {
        self.0.remove(dependency);
    }

    pub fn get(&self, dependency : &ArtifactDependency) -> Option<&Artifact> {
        self.0.get(dependency)
    }

    pub fn resolved(&self, dependency : &ArtifactDependency) -> bool {
        self.0.contains_key(dependency)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

}


impl TryFrom<ArtifactDependencyResolutions> for ArtifactResolutions {

    type Error = anyhow::Error;

    fn try_from(dependency_resolutions : ArtifactDependencyResolutions) -> Result<ArtifactResolutions, anyhow::Error> {
        let mut resolutions = ArtifactResolutions::new();

        for (outer_dependency, artifact) in dependency_resolutions.0.iter() {
            resolutions.register(artifact.clone());
            for dependency in artifact.dependencies.iter() {
                let resolution = dependency_resolutions.0.get(&dependency);
                match resolution {
                    Some(resolution) => {
                        resolutions.add(artifact.clone(), resolution.clone());
                    },
                    None => {
                        anyhow::bail!("Invalid dependency resolution. Could not resolve dependency: {:?} for {:?}", dependency, outer_dependency);
                    }
                }
            }
        }

        Ok(resolutions)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactResolutions(pub BTreeMap<Artifact, BTreeSet<Artifact>>);

impl ArtifactResolutions {

    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn register(&mut self, artifact : Artifact) {
        self.0.entry(artifact).or_insert_with(BTreeSet::new);
    }

    pub fn add(&mut self, artifact : Artifact, dependency : Artifact) {
        self.0.entry(artifact).or_insert_with(BTreeSet::new).insert(dependency);
    }

    pub fn remove(&mut self, artifact : &Artifact) {
        self.0.remove(artifact);
    }

    pub fn remove_dependency(&mut self, artifact : &Artifact, dependency : &Artifact) {
        if let Some(dependencies) = self.0.get_mut(artifact) {
            dependencies.remove(dependency);
        }
    }

    /// Helper function to perform DFS and check for cycles
    fn is_cyclic_util(
        &self, 
        node: &Artifact, 
        visited: &mut BTreeSet<Artifact>, 
        stack: &mut BTreeSet<Artifact>
    ) -> bool {
        if !visited.contains(node) {
            // Mark the current node as visited and part of recursion stack
            visited.insert(node.clone());
            stack.insert(node.clone());

            // Recurse for all the artifacts dependent on this node
            for dependent in self.0.get(node).unwrap_or(&BTreeSet::new()).iter() {
                if !visited.contains(dependent) && self.is_cyclic_util(dependent, visited, stack) {
                    return true;
                } else if stack.contains(dependent) {
                    // If the node is in the recursion stack, then there is a cycle
                    return true;
                }
            }
        }

        // Remove the node from recursion stack
        stack.remove(node);
        false
    }

    /// Function to check if the graph contains a cycle
    pub fn has_cycles(&self) -> bool {
        let mut visited = BTreeSet::new();
        let mut stack = BTreeSet::new();

        // Call the recursive helper function to detect cycle in different DFS trees
        for artifact in self.0.keys() {
            if !visited.contains(artifact) && self.is_cyclic_util(artifact, &mut visited, &mut stack) {
                return true;
            }
        }

        false
    }

    // DFS function to find the maximum dependent depth
    fn find_max_dependent_depth(&self, node: &Artifact, depth_map: &mut BTreeMap<&Artifact, usize>) -> usize {

        if let Some(dependencies) = self.0.get(node) {
            if dependencies.is_empty() {
                return 0;
            }

            let depths = dependencies
                .iter()
                .map(|dep| {
                    depth_map.get(dep)
                        .copied()
                        .unwrap_or_else(|| self.find_max_dependent_depth(dep, depth_map))
                })
                .collect::<Vec<usize>>();

            *depths.iter().max().unwrap() + 1
        } else {
            0
        }
    }

    pub fn get_all_dependent_depths(&self) -> BTreeMap<&Artifact, usize> {
        let mut depth_map = BTreeMap::new();

        for artifact in self.0.keys() {
            let depth = self.find_max_dependent_depth(artifact, &mut depth_map);
            depth_map.insert(artifact, depth);
        }

        depth_map
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArtifactResolutionPlan(pub Vec<BTreeSet<Artifact>>);

impl ArtifactResolutionPlan {

    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, artifacts : BTreeSet<Artifact>) {
        self.0.push(artifacts);
    }

    pub fn reverse(&mut self) {
        self.0.reverse();
    }

}

impl TryFrom<ArtifactResolutions> for ArtifactResolutionPlan {
    type Error = anyhow::Error;

    fn try_from(resolutions: ArtifactResolutions) -> Result<Self, Self::Error> {
        if resolutions.has_cycles() {
            anyhow::bail!("Dependency graph contains cycles.");
        }

        let mut plan = Vec::new();

        // Get the max dependency depths for each artifact
        let depth_map = resolutions.get_all_dependent_depths();

        for (artifact, depth) in depth_map.iter() {

            while *depth >= plan.len() {
                plan.push(BTreeSet::new());
            }

            plan[*depth].insert((*artifact).clone());

        }

        // Since we started from the leaves
        Ok(ArtifactResolutionPlan(plan))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtifactResolutionStatus {
    User,
    Resolved,
    Forced,

    // to be used when querying for artifacts
    Any
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub struct ArtifactResolution {
    pub artifact : Artifact,
    pub status : ArtifactResolutionStatus
}

impl ArtifactResolution {

    pub fn is_any(&self) -> bool {
        match self.status {
            ArtifactResolutionStatus::Any => true,
            _ => false
        }
    }

}

impl From<ArtifactResolution> for Artifact {
    fn from(artifact_installation : ArtifactResolution) -> Self {
        artifact_installation.artifact
    }
}

impl PartialEq for ArtifactResolution {
    fn eq(&self, other: &Self) -> bool {
        match self.status {
            ArtifactResolutionStatus::Any => self.artifact == other.artifact,
            _ => {
                self.status == other.status 
                && self.artifact == other.artifact
            }
        }
    }
}

impl Eq for ArtifactResolution {}


#[cfg(test)]
pub mod test {

    use super::*;


    #[tokio::test]
    async fn test_big_bang() -> Result<(), anyhow::Error> {

        let big_bang = Artifact::test()
        .with_name("big-bang".to_string());
        let universe = Artifact::test()
        .with_name("universe".to_string());

        let mut resolutions = ArtifactResolutions::new();
        resolutions.register(big_bang.clone());
        resolutions.add(universe.clone(), big_bang.clone());

        let expected_plan = ArtifactResolutionPlan(
            vec![
                vec![
                    big_bang.clone()
                ].into_iter().collect(),
                vec![
                    universe.clone()
                ].into_iter().collect()
            ]
        );

        let plan = ArtifactResolutionPlan::try_from(resolutions)?;

        assert_eq!(plan, expected_plan);

        Ok(())

    }

    #[tokio::test]
    pub async fn test_big_bang_to_earth() -> Result<(), anyhow::Error> {

        // big bang creates the universe
        let big_bang = Artifact::test()
        .with_name("big-bang".to_string());

        // universe creates atoms
        let universe = Artifact::test()
        .with_name("universe".to_string());

        // hydrogen creates stars
        let hydrogen = Artifact::test()
        .with_name("hydrogen".to_string());

        // stars create metals for planets
        let stars = Artifact::test()
        .with_name("stars".to_string());

        // metals create planets
        let metals = Artifact::test()
        .with_name("metals".to_string());

        // oxygen with hydrogen creates water
        let oxygen = Artifact::test()
        .with_name("oxygen".to_string());

        // water with metals creates earth
        let water = Artifact::test()
        .with_name("water".to_string());

        // earth
        let earth = Artifact::test()
        .with_name("earth".to_string());

        let mut resolutions = ArtifactResolutions::new();
        resolutions.register(big_bang.clone());
        resolutions.add(universe.clone(), big_bang.clone());
        resolutions.add(hydrogen.clone(), universe.clone());
        resolutions.add(stars.clone(), hydrogen.clone());
        resolutions.add(metals.clone(), stars.clone());
        resolutions.add(oxygen.clone(), universe.clone());
        resolutions.add(water.clone(), hydrogen.clone());
        resolutions.add(water.clone(), oxygen.clone());
        resolutions.add(earth.clone(), metals.clone());
        resolutions.add(earth.clone(), water.clone());

        let expected_plan = ArtifactResolutionPlan(
            vec![
                vec![
                    big_bang.clone()
                ].into_iter().collect(),
                vec![
                    universe.clone()
                ].into_iter().collect(),
                vec![
                    hydrogen.clone(),
                    oxygen.clone()
                ].into_iter().collect(),
                vec![
                    stars.clone(),
                    water.clone()
                ].into_iter().collect(),
                vec![
                    metals.clone()
                ].into_iter().collect(),
                vec![
                    earth.clone()
                ].into_iter().collect()
            ]
        );

        let plan = ArtifactResolutionPlan::try_from(resolutions)?;

        assert_eq!(plan, expected_plan);

        Ok(())

    }

}
