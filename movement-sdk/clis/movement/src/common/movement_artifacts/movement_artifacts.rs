use serde::{Serialize, Deserialize};
use super::movement_cli::MovementCliArtifact;
use super::m1::M1Artifact;
use super::super::movement_releases::Release;
use crate::common::util::Version;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub release_target_paths : Vec<String>, // ! this needs to be string instead of PathBuf to accommodate complexity
    pub artifact_target_paths : Vec<String> // ! this needs to be string instead of PathBuf to accommodate complexity
};

impl Location {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn push(&mut self, value : String) {
        self.0.push(value);
    }
    pub fn get(&self) -> Vec<String> {
        self.0.clone()
    }
}

#[async_trait::async_trait]
pub trait BuilderOperations {

    async fn build(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error>;

    async fn remove(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error>;

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Builder {
    KnownScript, 
    RustBuildRelease,
    RustBuildDebug, 
    FromArchive,
    ReleaseOnly,
}

impl BuilderOperations for Builder {

    async fn build(&self, release : &Release, version : &Version, location : &Location) -> Result<(), anyhow::Error> {
        todo!()
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub release : Release,
    pub location : Location,
    pub version : Version,
    pub builder : Builder
}

impl Artifact {

    pub fn new(release : Release, location : Location, version : Version, builder : Builder) -> Self {
        Self {
            release,
            location,
            version,
            builder
        }
    }

    async fn install(&self) -> Result<(), anyhow::Error> {

        self.builder.build(&self.release, &self.version, &self.location).await?;
        Ok(())

    }

    async fn uninstall(&self) -> Result<(), anyhow::Error> {
        
        self.builder.remove(&self.release, &self.version, &self.location).await?;
        Ok(())

    }

}