use serde::{Serialize, Deserialize};
use super::movement_cli::MovementCliArtifact;
use super::m1::M1Artifact;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MovementArtifact {
    Movement(MovementCliArtifact),
    M1Artifact(M1Artifact)
}