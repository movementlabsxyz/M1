use super::{Builder, BuilderOperations};
use crate::util::artifact::{
    Artifact,
    resolution::ArtifactDependencyResolutions
};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use zip_extensions::read::ZipArchiveExtensions;
use crate::util::util::fs;
use flate2::read::GzDecoder;
use crate::util::release::ReleaseOperations;
use crate::location::Location;
use crate::movement_dir::MovementDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Unarchive {
    TarGz,
    Tar,
    Zip,
    Unknown
}

impl Unarchive {

    pub async fn unarchive(&self, source : &PathBuf, destination : &PathBuf) -> Result<(), anyhow::Error> {


        let source = source.clone();
        let destination = destination.clone();

        match self {
            Unarchive::TarGz => {

                tokio::task::spawn_blocking(move || {

                    let file = File::open(source)?;
                    let reader = BufReader::new(file);
                    let mut archive = tar::Archive::new(GzDecoder::new(reader));

                    // Unpack the TAR archive to the destination
                    archive.unpack(destination)?;

                    Ok::<(), anyhow::Error>(())

                }).await??;

            }, 
            Unarchive::Tar => {

                tokio::task::spawn_blocking(move || {
                        
                    let file = File::open(source)?;
                    let reader = BufReader::new(file);
                    let mut archive = tar::Archive::new(reader);

                    // Unpack the TAR archive to the destination
                    archive.unpack(destination)?;

                    Ok::<(), anyhow::Error>(())

                }).await??;
        
                
            },
            Unarchive::Zip => {

                tokio::task::spawn_blocking(move || {

                    let file = File::open(source)?;
                    let mut archive = zip::ZipArchive::new(file)?;

                    // UNpack the ZIP archive to the destination
                    archive.extract(destination)?;

                    Ok::<(), anyhow::Error>(())

                }).await??;

            },
            _ => {
                anyhow::bail!("Cannot unarchive an unsupported archive type.");
            }
        };

        Ok(())

    }

}

#[async_trait::async_trait]
impl BuilderOperations for Unarchive {

    async fn build(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        // download the release to a tempdir
        let temp_dir = tempfile::tempdir()?;
        let path = temp_dir.path().to_path_buf();
        let tmp_location = path.join("tmp");
        let location = artifact.release.get(&tmp_location.into()).await?;

        let destination  = match &artifact.location {
            Location::Path(path) => {
                Ok::<PathBuf, anyhow::Error>(movement.path.join(path))
            },
            _ => {
                anyhow::bail!("Failed to build artifact.");
            }
        }?;

        // mkdir -p
        match path.parent() {
            Some(parent) => {
                tokio::fs::create_dir_all(parent).await?;
            },
            None => {
                anyhow::bail!("Failed to build artifact not located in parent dir.");
            }
        };

        // unarchive the release
        match location {
            Location::Path(path) => {
                self.unarchive(&path, &destination).await?;
            },
            _ => {
                anyhow::bail!("Cannot unarchive an archive from a non-path location.");
            }
        };
        
        Ok(artifact.clone())

    }

    async fn remove(&self, artifact : &Artifact, movement : &MovementDir) -> Result<Artifact, anyhow::Error> {

        let destination  = match &artifact.location {
            Location::Path(path) => {
                Ok::<PathBuf, anyhow::Error>(movement.path.join(path))
            },
            _ => {
                anyhow::bail!("Failed to build artifact.");
            }
        }?;

        fs::remove(&destination).await?;

        Ok(artifact.clone())

    }

}

impl Into<Builder> for Unarchive {
    fn into(self) -> Builder {
        Builder::Unarchive(self)
    }
}

#[cfg(test)]
pub mod test {

    #[tokio::test]
    async fn test_known_script() -> Result<(), anyhow::Error> {

        Ok(())

    }

}