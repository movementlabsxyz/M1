pub mod api;
pub mod block;
pub mod state;
pub mod vm;

use std::io;

use avalanche_types::subnet;
use tokio::sync::broadcast::{self, Receiver, Sender};

pub async fn run_subnet() -> io::Result<()> {
    let (stop_ch_tx, stop_ch_rx): (Sender<()>, Receiver<()>) = broadcast::channel(1);
    let vm_server = subnet::rpc::vm::server::Server::new(vm::Vm::new(), stop_ch_tx);
    log::info!("readying server");
    subnet::rpc::vm::serve(vm_server, stop_ch_rx).await
}