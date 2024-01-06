pub mod release;

// various release types
pub mod file_release;
pub mod http_get_release;
pub mod movement_github_platform_release;
pub mod movement_github_release;

pub use release::*;
pub mod releases;
