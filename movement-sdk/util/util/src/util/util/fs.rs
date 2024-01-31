use std::path::PathBuf;
use tokio::fs;

pub async fn remove(path : &PathBuf) -> Result<(), anyhow::Error> {
    let metadata = fs::metadata(&path).await?;
    if metadata.is_file() {
        // Remove the file
        fs::remove_file(&path).await?;
    } else if metadata.is_dir() {
        // Remove the directory and its contents
        fs::remove_dir_all(&path).await?;
    } else {
        // Handle symbolic links, special files, etc., if necessary
        // ...
    }
    Ok(())
}