use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum M1Artifact {
    M1Source,
    M1SourceWithSubmodules,
    M1SubnetBinary,
    M1SubnetRequestProxySource
}