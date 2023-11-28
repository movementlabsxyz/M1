#![forbid(unsafe_code)]

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use clap::Parser;
use std::process::exit;

#[tokio::main]
async fn main() {
   
   // todo: parse clap 

}