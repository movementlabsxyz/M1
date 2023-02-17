pub mod tx_heap;

use std::{
    collections::VecDeque,
    io::{Error, ErrorKind, Result},
    sync::{Arc, RwLock},
};

use avalanche_types::ids;
use tokio::sync::broadcast;

use crate::chain::tx::tx::Transaction;

use self::tx_heap::{Entry, TxHeap};

pub struct Mempool {
    inner: Arc<RwLock<MempoolInner>>,
    max_size: u64,
}

pub struct MempoolInner {
    new_txs: Vec<Transaction>,
    max_heap: TxHeap,
    min_heap: TxHeap,

    /// Channel of length one, which the mempool ensures has an item on
    /// it as long as there is an unissued transaction remaining in [txs].
    pending_tx: broadcast::Sender<()>,
    pub pending_rx: broadcast::Receiver<()>,
}

impl Mempool {
    pub fn new(max_size: u64) -> Self {
        // initialize channel
        let (pending_tx, pending_rx): (broadcast::Sender<()>, broadcast::Receiver<()>) =
            broadcast::channel(1);
        Self {
            inner: Arc::new(RwLock::new(MempoolInner {
                new_txs: Vec::new(),
                max_heap: TxHeap::new(max_size as usize, false),
                min_heap: TxHeap::new(max_size as usize, true),
                pending_tx,
                pending_rx,
            })),
            max_size,
        }
    }

    /// Returns a broadcast receiver for the pending tx channel.
    pub fn subscribe_pending(&self) -> broadcast::Receiver<()> {
        let inner = self.inner.read().unwrap();
        inner.pending_tx.subscribe()
    }

    /// Returns Tx from Id if it exists.
    pub fn get(&self, id: &ids::Id) -> Result<Option<Transaction>> {
        let inner = self.inner.read().unwrap();
        if let Some(entry) = inner.max_heap.get(id) {
            if let Some(tx) = entry.tx {
                return Ok(Some(tx));
            }
        }
        Ok(None)
    }

    /// Adds a Tx Entry to mempool and writes to the pending channel.
    pub fn add(&self, tx: &Transaction) -> Result<bool> {
        log::debug!("add: called");
        let tx_id = &tx.id;

        let mut inner = self.inner.write().unwrap();

        // don't add duplicates
        if inner.max_heap.has(tx_id) {
            log::debug!("add: found duplicate");
            return Ok(false);
        }
        let old_len = inner.max_heap.len();

        let entry = &Entry {
            id: tx_id.to_owned(),
            tx: Some(tx.clone()),
            index: old_len,
        };

        // optimistically add tx to mempool
        inner.max_heap.push(entry);
        inner.min_heap.push(entry);

        // price is not supported so for now use FIFO
        while inner.max_heap.len() > self.max_size as usize {
            if let Some(tx) = inner.min_heap.pop_front() {
                if tx.id == *tx_id {
                    log::debug!("add: tx id weird");
                    return Ok(false);
                }
            }
        }

        inner.new_txs.push(tx.to_owned());

        inner.pending_tx.send(()).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to send pending tx: {}", e),
            )
        })?;
        log::debug!("add: pending tx sent");

        Ok(true)
    }

    /// Pops the last entry from the list. Returns None if empty.
    pub fn pop_min(&self) -> Option<Transaction> {
        let mut inner = self.inner.write().unwrap();

        if let Some(entry) = inner.min_heap.pop_front() {
            return entry.tx;
        }

        None
    }

    /// Pops the first entry from the list. Returns None if empty.
    pub fn pop_back(&self) -> Option<Transaction> {
        let mut inner = self.inner.write().unwrap();

        if let Some(entry) = inner.max_heap.pop_back() {
            return entry.tx;
        }

        None
    }

    /// Returns len of mempool data.
    pub fn len(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.max_heap.len()
    }

    pub fn is_empty(&self) -> bool {
        let inner = self.inner.read().unwrap();
        inner.max_heap.is_empty()
    }

    // TODO: remove when batch and iterator support is added for version db.
    /// Returns a copy of mempool txs.
    pub fn get_txs(&self) -> VecDeque<Entry> {
        let inner = self.inner.read().unwrap();
        let txs = inner.max_heap.items.clone();
        txs
    }

    /// Returns the vec of transactions ready to gossip and replaces it with an empty vec.
    pub fn new_txs(&mut self, max_units: u64) -> Result<Vec<Transaction>> {
        let mut inner = self.inner.write().unwrap();
        log::debug!("new_txs: found: {}", inner.new_txs.len());

        let mut selected: Vec<Transaction> = Vec::new();
        let mut units = self.max_size;

        // It is possible that a block may have been accepted that contains some
        // new transactions before [new_txs] is called.
        for (i, tx) in inner.new_txs.iter().cloned().enumerate() {
            log::debug!("new_txs: found a tx");
            if !inner.max_heap.has(&tx.id) {
                log::debug!("new_txs: already have tx: skipping");
                continue;
            }

            if 1 > max_units - units {
                inner.new_txs = inner.new_txs[i..].to_vec();
                return Ok(selected);
            }

            units += 1;
            log::debug!("pushed selected");
            selected.push(tx);
        }
        // reset
        inner.new_txs = Vec::new();

        Ok(selected)
    }

    /// Prunes any Ids not included in valid hashes set.
    pub fn prune(&self, valid_hashes: ids::Set) {
        let mut to_remove: Vec<ids::Id> = Vec::with_capacity(valid_hashes.len());
        let inner = self.inner.write().unwrap();

        for max_entry in inner.max_heap.items.iter() {
            if let Some(tx) = &max_entry.tx {
                if !valid_hashes.contains(&tx.id) {
                    to_remove.push(max_entry.id);
                }
            }
        }
        drop(inner);

        for id in to_remove.iter() {
            log::debug!("attempting to prune id: {}", id);
            if self.remove(id.to_owned()).is_some() {
                log::debug!("id deleted: {}", id);
            } else {
                log::debug!("failed to delete id: {}: not found", id);
            }
        }
    }

    /// Removes Tx entry from mempool data if it exists.
    pub fn remove(&self, id: ids::Id) -> Option<Transaction> {
        let mut inner = self.inner.write().unwrap();

        // TODO: try to optimize.
        // find the position of the entry in vec and remove
        match inner.max_heap.items.iter().position(|e| e.id == id) {
            Some(index) => {
                inner.max_heap.items.remove(index);
            }
            None => return None,
        }

        // remove entry from lookup
        match inner.max_heap.lookup.remove(&id) {
            Some(_) => {}
            None => {
                log::error!("unexpected mempool imbalance");
                return None;
            }
        }

        // min

        // remove entry from lookup
        match inner.max_heap.lookup.remove(&id) {
            Some(_) => {}
            None => {
                log::error!("unexpected mempool imbalance");
                return None;
            }
        }

        match inner.min_heap.items.iter().position(|e| e.id == id) {
            Some(index) => {
                if let Some(txe) = inner.min_heap.items.remove(index) {
                    return txe.tx;
                }
                None
            }
            None => return None,
        }
    }
}

#[tokio::test]
async fn test_mempool() {
    use crate::chain::tx::{decoder, tx::TransactionType, unsigned};

    // init mempool
    let mempool = Mempool::new(10);
    let mut pending_rx = mempool.subscribe_pending();

    // create tx_1
    let tx_data_1 = unsigned::TransactionData {
        typ: TransactionType::Claim,
        space: "foo".to_string(),
        key: "".to_string(),
        value: vec![],
    };
    let resp = tx_data_1.decode();
    assert!(resp.is_ok());
    let utx_1 = resp.unwrap();
    let secret_key = avalanche_types::key::secp256k1::private_key::Key::generate().unwrap();
    let dh_1 = decoder::hash_structured_data(&utx_1.typed_data().await).unwrap();
    let sig_1 = secret_key.sign_digest(dh_1.as_bytes()).unwrap();
    let tx_1 = Transaction::new(utx_1, sig_1.to_bytes().to_vec());

    // add tx_1 to mempool
    let tx_1_id = tx_1.id;
    assert_eq!(mempool.add(&tx_1).unwrap(), true);
    // drain channel
    let _ = pending_rx.recv().await.unwrap();
    assert_eq!(mempool.len(), 1);

    // add tx_1 as valid
    let mut valid_txs = ids::new_set(2);
    valid_txs.insert(tx_1_id);

    // create tx_2
    let tx_data_2 = unsigned::TransactionData {
        typ: TransactionType::Claim,
        space: "bar".to_string(),
        key: "".to_string(),
        value: vec![],
    };
    let resp = tx_data_2.decode();
    assert!(resp.is_ok());
    let utx_2 = resp.unwrap();
    let dh_2 = decoder::hash_structured_data(&utx_2.typed_data().await).unwrap();
    let sig_2 = secret_key.sign_digest(dh_2.as_bytes()).unwrap();
    let mut tx_2 = Transaction::new(utx_2, sig_2.to_bytes().to_vec());
    tx_2.id = ids::Id::from_slice("sup".as_bytes());

    // add tx_2 to mempool
    assert_eq!(mempool.add(&tx_2).unwrap(), true);
    assert_eq!(mempool.len(), 2);

    // drain channel
    let _ = pending_rx.recv().await.unwrap();

    // prune tx_2 as invalid
    mempool.prune(valid_txs);

    // verify one tx entry removed
    assert_eq!(mempool.len(), 1);

    // verify tx_1 exists
    let resp = mempool.get(&tx_1_id);
    assert!(resp.is_ok());

    assert_eq!(resp.unwrap().unwrap().id, tx_1_id);
}

#[tokio::test]
async fn test_mempool_threads() {
    use crate::chain::tx::{decoder, tx::TransactionType, unsigned};
    use tokio::time::sleep;

    let vm = crate::vm::ChainVm::new();

    let inner = Arc::clone(&vm.inner);
    tokio::spawn(async move {
        let vm_inner = inner.write().await;
        let tx_data_1 = unsigned::TransactionData {
            typ: TransactionType::Claim,
            space: "foo".to_string(),
            key: "".to_string(),
            value: vec![],
        };
        let resp = tx_data_1.decode();
        assert!(resp.is_ok());
        let utx = resp.unwrap();
        let secret_key = avalanche_types::key::secp256k1::private_key::Key::generate().unwrap();
        let dh = decoder::hash_structured_data(&utx.typed_data().await).unwrap();
        let sig = secret_key.sign_digest(&dh.as_bytes()).unwrap();
        let tx = Transaction::new(utx, sig.to_bytes().to_vec());

        // add tx to mempool
        let resp = vm_inner.mempool.add(&tx);
        assert!(resp.is_ok());
    });

    let inner = Arc::clone(&vm.inner);
    tokio::spawn(async move {
        sleep(std::time::Duration::from_micros(10)).await;
        let vm_inner = inner.read().await;
        // check that inner mempool has been updated in the other thread
        assert_eq!(vm_inner.mempool.len(), 1);
    });

    // wait for threads
    sleep(std::time::Duration::from_millis(10)).await;
}
