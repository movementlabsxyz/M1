use std::{
    fmt,
    fs::{self, File},
    io::{self, Error, ErrorKind, Write},
    path::Path,
};

use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct Genesis {
    pub author: String,
    pub welcome_message: String,
}

impl Default for Genesis {
    fn default() -> Self {
        Self::default()
    }
}

impl Genesis {
    pub fn default() -> Self {
        Self {
            author: String::from("subnet creator"),
            welcome_message: String::from("Hello from Rust VM!"),
        }
    }

    pub fn from_json<S>(d: S) -> io::Result<Self>
    where
        S: AsRef<[u8]>,
    {
        let resp: Self = match serde_json::from_slice(d.as_ref()) {
            Ok(p) => p,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to decode {}", e),
                ));
            }
        };
        Ok(resp)
    }

    pub fn sync(&self, file_path: &str) -> io::Result<()> {
        info!("syncing genesis to '{}'", file_path);
        let path = Path::new(file_path);
        let parent_dir = path.parent().unwrap();
        fs::create_dir_all(parent_dir)?;

        let ret = serde_json::to_vec(&self);
        let d = match ret {
            Ok(d) => d,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("failed to serialize genesis info to YAML {}", e),
                ));
            }
        };
        let mut f = File::create(&file_path)?;
        f.write_all(&d)?;

        Ok(())
    }
}

impl fmt::Display for Genesis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_yaml::to_string(&self).unwrap();
        write!(f, "{}", s)
    }
}
