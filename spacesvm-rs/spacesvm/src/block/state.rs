use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
    num::NonZeroUsize,
    sync::Arc,
};

use avalanche_types::{
    choices::status::{self, Status},
    hash, ids,
    subnet::{self, rpc::database::errors},
};
use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::chain::{
    self,
    storage::{prefix_block_key, prefix_tx_value_key},
    tx::Transaction,
};

use super::{Block, BLOCKS_LRU_SIZE};

const LAST_ACCEPTED_BLOCK_KEY: &[u8] = b"last_accepted";
pub const BYTE_DELIMITER: &[u8] = b"/";
pub const HASH_LEN: usize = ids::LEN + 2;

#[derive(Serialize, Deserialize, Default)]
pub struct ValueMeta {
    size: u64,
    tx_id: ids::Id,
}

#[derive(Serialize, Deserialize, Default)]
pub struct BlockWrapper {
    block: Vec<u8>,
    status: Status,
}

#[derive(Default)]
pub struct State {
    inner: Arc<RwLock<StateInner>>,
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Default for StateInner {
    // Memdb by default
    fn default() -> StateInner {
        StateInner {
            db: subnet::rpc::database::memdb::Database::new(),
            last_accepted: ids::Id::empty(),
            verified_blocks: HashMap::new(),
            accepted_blocks: LruCache::new(NonZeroUsize::new(BLOCKS_LRU_SIZE).unwrap()),
        }
    }
}

pub struct StateInner {
    db: Box<dyn subnet::rpc::database::Database + Send + Sync>,
    /// The last accepted block by this VM
    last_accepted: ids::Id,
    /// Blocks that have been verified but not yet accepted
    verified_blocks: HashMap<ids::Id, Block>,
    /// Cache of accepted blocks
    accepted_blocks: LruCache<ids::Id, Block>,
}

impl State {
    pub fn new(db: Box<dyn subnet::rpc::database::Database + Send + Sync>) -> Self {
        return Self {
            inner: Arc::new(RwLock::new(StateInner {
                db,
                verified_blocks: HashMap::new(),
                last_accepted: ids::Id::empty(),
                accepted_blocks: LruCache::new(NonZeroUsize::new(BLOCKS_LRU_SIZE).unwrap()),
            })),
        };
    }

    /// Returns block from LRU cache or None if not found.
    pub async fn get_accepted_block(&self, id: ids::Id) -> Option<Block> {
        let mut inner = self.inner.write().await;

        if let Some(block) = inner.accepted_blocks.get(&id) {
            return Some(block.to_owned());
        }

        None
    }

    /// Adds an accepted block to the LRU cache. Returns the old value of the block
    /// if a key is updated. If the key is new it returns None.
    pub async fn set_accepted_block(&self, id: ids::Id, block: &Block) -> Result<Option<Block>> {
        let mut inner = self.inner.write().await;

        Ok(inner.accepted_blocks.put(id, block.to_owned()))
    }

    /// Returns block from map or None if not found.
    pub async fn get_verified_block(&self, id: ids::Id) -> Option<Block> {
        let inner = self.inner.read().await;

        if let Some(block) = inner.verified_blocks.get(&id) {
            return Some(block.to_owned());
        }

        None
    }

    /// Adds a verified block to the verified_blocks hash. Returns the old value of the block
    /// if a key is updated. If the key is new it returns None.
    pub async fn set_verified_block(&self, block: Block) -> Result<Option<Block>> {
        let mut inner = self.inner.write().await;

        Ok(inner.verified_blocks.insert(block.id, block))
    }

    /// Remove verified block from the verified_blocks hash. Returns the block if it existed and
    /// otherwise None.
    pub async fn remove_verified_block(&self, id: ids::Id) -> Result<Option<Block>> {
        let mut inner = self.inner.write().await;

        Ok(inner.verified_blocks.remove(&id))
    }

    /// Persists last accepted block Id into database.
    pub async fn set_last_accepted(&self, block: &mut Block) -> Result<()> {
        let block_id = block.id;

        // persist last_accepted Id to database with fixed key
        let mut inner = self.inner.write().await;

        log::debug!("set_last_accepted: key value: {:?}", block_id.to_vec());
        inner
            .db
            .put(LAST_ACCEPTED_BLOCK_KEY, &block_id.to_vec())
            .await
            .map_err(|e| {
                Error::new(
                    ErrorKind::Other,
                    format!("failed to put last accepted block: {}", e),
                )
            })?;

        for tx in block.txs.iter_mut() {
            if is_set_tx(&tx).await {
                let maybe_value = &tx.unsigned_transaction.get_value().await;
                if maybe_value.is_none() {
                    continue;
                }
                let value = maybe_value.as_ref().unwrap();
                log::debug!(
                    "set_last_accepted: put prefix_tx_value_key: {:?}\n",
                    prefix_tx_value_key(&tx.id)
                );
                log::debug!(
                    "set_last_accepted: put prefix_tx_value_key value: {:?}\n",
                    value
                );

                inner
                    .db
                    .put(&prefix_tx_value_key(&tx.id), &value)
                    .await
                    .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

                tx.unsigned_transaction.set_value(tx.id.to_vec()).await? // used to properly parse on restore
            }
        }

        let bytes = &serde_json::to_vec(&block)?;

        inner
            .db
            .put(&prefix_block_key(&block_id), &bytes)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    /// Attempts to retrieve the last accepted block and return the corresponding
    /// block Id. If not the key is found returns Id::empty().
    pub async fn get_last_accepted(&self) -> Result<ids::Id> {
        let inner = self.inner.read().await;

        if !inner.last_accepted.is_empty() {
            return Ok(inner.last_accepted);
        }

        match inner.db.get(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(value) => {
                let block_id = ids::Id::from_slice(&value);
                Ok(block_id)
            }
            Err(e) => {
                if errors::is_not_found(&e) {
                    return Ok(ids::Id::empty());
                }
                return Err(e);
            }
        }
    }

    /// Attempts to return block on disk state.
    pub async fn get_block(&self, block_id: ids::Id) -> Result<Block> {
        log::debug!("get_block: called");
        let mut inner = self.inner.write().await;

        if let Some(b) = inner.accepted_blocks.get(&block_id) {
            log::debug!("get_block: found accepted block");
            return Ok(b.clone());
        }

        let block_bytes = inner.db.get(&prefix_block_key(&block_id)).await?;
        let mut block: Block = serde_json::from_slice(&block_bytes)?;

        //  restore the unlinked values associated with all set_tx.value
        for tx in block.txs.iter_mut() {
            if is_set_tx(&tx).await {
                if let Some(tx_value) = tx.unsigned_transaction.get_value().await {
                    let tx_id = &ids::Id::from_slice(&tx_value);

                    let value = inner
                        .db
                        .get(&prefix_tx_value_key(tx_id))
                        .await
                        .map_err(|e| {
                            Error::new(
                                ErrorKind::Other,
                                format!("failed to get tx value key: {}", e),
                            )
                        })?;
                    tx.unsigned_transaction.set_value(value).await?
                } else {
                    continue;
                }
            }
        }

        // parse block inline
        let bytes = &serde_json::to_vec(&block)?;
        block.bytes = bytes.to_vec();
        block.id = ids::Id::from_slice(hash::keccak256(bytes).as_bytes());
        block.st = status::Status::Accepted;
        block.state = self.clone();

        for tx in block.txs.iter_mut() {
            tx.init().await?;
        }

        log::debug!("get_block: {:?}", block);
        Ok(block)
    }

    /// Attempts to parse a byte array into a block. If the source is empty
    /// bytes will be marshalled from a default block.
    pub async fn parse_block(
        &self,
        maybe_block: Option<Block>,
        mut source: Vec<u8>,
        status: Status,
    ) -> Result<Block> {
        let mut block: Block;
        if maybe_block.is_none() {
            block = Block::default();
        } else {
            block = maybe_block.unwrap();
        }

        if source.is_empty() {
            source = serde_json::to_vec(&block)?;
        }
        block.bytes = source.to_vec();
        block.id = ids::Id::from_slice(hash::keccak256(source).as_bytes());

        block.st = status;
        block.state = self.clone();

        for tx in block.txs.iter_mut() {
            tx.init().await?;
        }

        Ok(block.to_owned())
    }

    /// Checks if the last accepted block key exists and returns true if has a value.
    pub async fn has_last_accepted(&self) -> Result<bool> {
        let inner = self.inner.read().await;

        match inner.db.has(LAST_ACCEPTED_BLOCK_KEY).await {
            Ok(has) => Ok(has),
            Err(err) => {
                if errors::is_not_found(&err) {
                    return Ok(false);
                }
                return Err(err);
            }
        }
    }

    pub async fn get_db(&self) -> Box<dyn subnet::rpc::database::Database + Send + Sync> {
        let inner = self.inner.read().await;
        inner.db.clone()
    }
}

async fn is_set_tx(tx: &chain::tx::tx::Transaction) -> bool {
    tx.unsigned_transaction.typ().await == chain::tx::tx::TransactionType::Set
}
