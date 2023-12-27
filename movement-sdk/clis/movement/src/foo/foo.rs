use clap::Subcommand;
use async_trait::async_trait;
use crate::util::cli::Command;

#[derive(Debug, Clone, Subcommand)]
pub enum Release {
    Left,
    Right
}

#[derive(Debug, Clone, Subcommand)]
pub enum Location {
    Top,
    Bottom
}

#[derive(Debug, Clone, Subcommand)]
pub enum Foo {
    Bar,
}
