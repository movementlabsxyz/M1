use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Arch {
    X86,
    X86_64,
    Arm,
    Aarch64,
    Mips,
    PowerPC,
    // ... other architectures as needed
}

impl Arch {
    pub fn current() -> Self {
        #[cfg(target_arch = "x86")]
        { Arch::X86 }

        #[cfg(target_arch = "x86_64")]
        { Arch::X86_64 }

        #[cfg(target_arch = "arm")]
        { Arch::Arm }

        #[cfg(target_arch = "aarch64")]
        { Arch::Aarch64 }

        #[cfg(target_arch = "mips")]
        { Arch::Mips }

        #[cfg(target_arch = "powerpc")]
        { Arch::PowerPC }

        // ... other architectures as needed
    }

    pub fn to_string(&self) -> String {
        match self {
            Arch::X86 => "x86".to_string(),
            Arch::X86_64 => "x86_64".to_string(),
            Arch::Arm => "arm".to_string(),
            Arch::Aarch64 => "aarch64".to_string(),
            Arch::Mips => "mips".to_string(),
            Arch::PowerPC => "powerpc".to_string(),
            // ... other architectures as needed
        }
    }
}
