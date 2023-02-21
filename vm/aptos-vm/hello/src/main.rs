use aptos_vm::{AptosVM};
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};
#[tokio::main]
async fn main() -> io::Result<()> {
    let count = AptosVM::get_num_proof_reading_threads();
    println!("pool count: {}", count);
    Ok(())
}