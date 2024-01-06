use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OS {
    Windows,
    MacOS,
    Linux,
    // ... other operating systems as needed
}

impl OS {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        { OS::Windows }

        #[cfg(target_os = "macos")]
        { OS::MacOS }

        #[cfg(target_os = "linux")]
        { OS::Linux }

        // ... other operating systems as needed
    }

    pub fn to_string(&self) -> String {
        match self {
            OS::Windows => "windows".to_string(),
            OS::MacOS => "macos".to_string(),
            OS::Linux => "linux".to_string(),
            // ... other operating systems as needed
        }
    }
}
