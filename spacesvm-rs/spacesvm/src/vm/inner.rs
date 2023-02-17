use avalanche_types::{ids, subnet};
use semver::Version;
use tokio::sync::mpsc;

use crate::{block, genesis::Genesis, mempool};

use super::MEMPOOL_SIZE;

pub struct Inner {
    pub ctx: Option<subnet::rpc::context::Context>,
    pub to_engine: Option<mpsc::Sender<subnet::rpc::common::message::Message>>,
    pub app_sender: Option<Box<dyn subnet::rpc::common::appsender::AppSender + Send + Sync>>,

    pub state: block::state::State,
    pub bootstrapped: bool,
    pub version: Version,
    pub genesis: Genesis,
    pub preferred: ids::Id,
    pub mempool: mempool::Mempool,
    pub block_status: block::builder::Status,

    pub builder_stop_rx: crossbeam_channel::Receiver<()>,
    pub builder_stop_tx: crossbeam_channel::Sender<()>,
    pub done_build_rx: crossbeam_channel::Receiver<()>,
    pub done_build_tx: crossbeam_channel::Sender<()>,
    pub done_gossip_rx: crossbeam_channel::Receiver<()>,
    pub done_gossip_tx: crossbeam_channel::Sender<()>,
    pub stop_rx: crossbeam_channel::Receiver<()>,
    pub stop_tx: crossbeam_channel::Sender<()>,
}

impl Inner {
    pub fn new() -> Self {
        let (stop_tx, stop_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (builder_stop_tx, builder_stop_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (done_build_tx, done_build_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);

        let (done_gossip_tx, done_gossip_rx): (
            crossbeam_channel::Sender<()>,
            crossbeam_channel::Receiver<()>,
        ) = crossbeam_channel::bounded(1);
        Self {
            ctx: None,
            to_engine: None,
            app_sender: None,

            // defaults
            state: block::state::State::default(),
            bootstrapped: false,
            version: Version::new(0, 0, 0),
            genesis: Genesis::default(),
            preferred: ids::Id::empty(),
            mempool: mempool::Mempool::new(MEMPOOL_SIZE),
            block_status: block::builder::Status::MayBuild,

            builder_stop_rx,
            builder_stop_tx,
            done_build_rx,
            done_build_tx,
            done_gossip_rx,
            done_gossip_tx,
            stop_rx,
            stop_tx,
        }
    }
}

impl Default for Inner {
    fn default() -> Self {
        Self::new()
    }
}
