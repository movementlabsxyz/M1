use std::io;

use avalanche_types::subnet;
use tokio::sync::broadcast::{self, Receiver, Sender};

mod vm;
mod state;
mod block;
mod api;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = broadcast::channel(1);
    let vm_server = subnet::rpc::vm::server::Server::new(vm::Vm::new(), stop_ch_tx);
    subnet::rpc::vm::serve(vm_server, stop_ch_rx).await
}
