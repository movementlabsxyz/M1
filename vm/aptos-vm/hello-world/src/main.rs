use hello_world::exe_transaction;
use aptos_logger::{info, Level, Logger};
fn main() {
    Logger::builder().level(Level::Debug).build();
    info!("hello {}", "world");
    exe_transaction()
}
