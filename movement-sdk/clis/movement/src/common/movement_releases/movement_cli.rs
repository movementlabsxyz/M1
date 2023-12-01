use serde::{Serialize, Deserialize};
use super::Release;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementGitHubReleases {
    movement_binary : Release,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MovementCliReleases {
    GitHub(MovementGitHubReleases)
}