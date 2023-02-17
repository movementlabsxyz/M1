use std::{
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
    sync::Arc,
    time::Duration,
};

use avalanche_types::ids::{self, Id};
use crossbeam_channel::TryRecvError;
use lru::LruCache;
use tokio::{sync::RwLock, time::sleep};

use crate::{chain, vm};

const GOSSIPED_TXS_LRU_SIZE: usize = 512;

// TODO: make configurable
const GOSSIP_INTERVAL: Duration = Duration::from_secs(10);
const REGOSSIP_INTERVAL: Duration = Duration::from_secs(30);
const TARGET_BLOCK_SIZE: u64 = 225;

pub struct Push {
    gossiped_tx: LruCache<Id, ()>,
    vm_inner: Arc<RwLock<vm::inner::Inner>>,
}

impl Push {
    pub fn new(vm_inner: Arc<RwLock<vm::inner::Inner>>) -> Self {
        let cache: LruCache<Id, ()> =
            LruCache::new(NonZeroUsize::new(GOSSIPED_TXS_LRU_SIZE).unwrap());
        Self {
            vm_inner,
            gossiped_tx: cache,
        }
    }

    pub async fn send_txs(&self, txs: Vec<chain::tx::tx::Transaction>) -> Result<()> {
        log::debug!("send_txs: called");
        if txs.is_empty() {
            log::debug!("send_txs: empty");
            return Ok(());
        }

        let b = serde_json::to_vec(&txs)
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to marshal txs: {}", e)))?;

        log::debug!("sending app gossip txs: {} size: {}", txs.len(), b.len());
        let vm = self.vm_inner.read().await;
        let appsender = vm
            .app_sender
            .as_ref()
            .ok_or_else(|| Error::new(ErrorKind::NotFound, "app_sender not found"))?;

        appsender
            .send_app_gossip(b)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("gossip txs failed: {}", e)))?;
        log::debug!("sending app gossip sent");
        Ok(())
    }

    pub async fn get_new_txs(&self) -> Result<Vec<chain::tx::tx::Transaction>> {
        let mut inner = self.vm_inner.write().await;
        log::debug!("get_new_txs: mempool len: {}", inner.mempool.len());
        inner.mempool.new_txs(TARGET_BLOCK_SIZE).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to get net tx from mempool: {}", e),
            )
        })
    }

    pub async fn gossip_new_txs(&mut self) -> Result<()> {
        log::debug!("gossip_new_txs: called");

        let new_txs = self.get_new_txs().await?;
        let mut txs: Vec<chain::tx::tx::Transaction> = Vec::with_capacity(new_txs.len());
        log::debug!("gossip_new_txs: len: {}", new_txs.len());

        for tx in new_txs.iter().cloned() {
            if self.gossiped_tx.contains(&tx.id) {
                log::debug!("already gossiped skipping id: {}", tx.id);
                continue;
            }

            self.gossiped_tx.put(tx.id, ());

            txs.push(tx);
        }

        self.send_txs(txs).await
    }

    /// Triggers "AppGossip" on the pending transactions in the mempool.
    /// "force" is true to re-gossip whether recently gossiped or not.
    pub async fn regossip_txs(&mut self) -> Result<()> {
        let mut txs: Vec<chain::tx::tx::Transaction> = Vec::new();
        let vm = self.vm_inner.read().await;

        let mempool = &vm.mempool;

        while !mempool.is_empty() {
            if let Some(tx) = mempool.pop_back() {
                // Note: when regossiping, we force resend even though we may have done it
                // recently.
                self.gossiped_tx.put(tx.id, ());
                txs.push(tx);
            }
        }

        self.send_txs(txs).await
    }

    pub async fn app_gossip(&mut self, node_id: ids::node::Id, message: &[u8]) -> Result<()> {
        log::debug!(
            "appgossip message handler, sender: {} bytes: {:?}",
            node_id,
            message
        );

        let mut txs: Vec<chain::tx::tx::Transaction> = serde_json::from_slice(message).unwrap();

        // submit incoming gossip
        log::debug!(
            "appgossip transactions are being submitted txs: {}",
            txs.len()
        );

        let vm = self.vm_inner.read().await;
        chain::storage::submit(&vm.state, &mut txs)
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("appgossip failed to submit txs peer_id: {}: {}", node_id, e),
                )
            })?;

        for tx in txs.iter_mut() {
            vm.mempool.add(tx).map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to add tx to mempool: {}", e),
                )
            })?;
        }

        Ok(())
    }

    pub async fn regossip(&mut self) {
        log::debug!("starting regossip loop");

        let inner = self.vm_inner.read().await;
        let stop_ch = inner.stop_rx.clone();
        drop(inner);

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            sleep(REGOSSIP_INTERVAL).await;
            log::debug!("tick regossip");

            let _ = self.regossip_txs().await;
        }

        log::debug!("shutdown regossip loop");
    }

    // Helper function initialize builder
    pub async fn init(&self) -> crossbeam_channel::Receiver<()> {
        let vm = self.vm_inner.read().await;
        vm.stop_rx.clone()
    }

    pub async fn gossip(&mut self) {
        log::debug!("starting gossip loops");
        let stop_ch = self.init().await;

        while stop_ch.try_recv() == Err(TryRecvError::Empty) {
            sleep(GOSSIP_INTERVAL).await;
            log::debug!("tick gossip");

            let _ = self.gossip_new_txs().await;
        }
    }
}
