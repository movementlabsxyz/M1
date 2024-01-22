use super::Version;
use crate::sys::{Arch, OS};
use std::path::PathBuf;

pub trait WithMovement {

    fn with_dir(self, path : &PathBuf) -> Self;

    fn with_version(self, version : &Version) -> Self;

    fn with_arch(self, arch : &Arch) -> Self;

    fn with_os(self, os : &OS) -> Self;

}