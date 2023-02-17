use std::{
    fmt::{self, Debug},
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{hash, ids, key, subnet};
use ethereum_types::Address;
use serde::{Deserialize, Serialize};

use crate::{block::Block, chain::storage::set_transaction};

use super::{decoder, unsigned::TransactionContext};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TransactionType {
    /// Root namespace.
    Claim,
    /// Create or update a key/value pair for a space.
    Set,
    /// Remove a key.
    Delete,
    /// Used for testing only
    Unknown,
}

impl Default for TransactionType {
    fn default() -> Self {
        TransactionType::Unknown
    }
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransactionType::Claim => write!(f, "claim"),
            TransactionType::Set => write!(f, "set"),
            TransactionType::Delete => write!(f, "delete"),
            TransactionType::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    pub unsigned_transaction: Box<dyn super::unsigned::Transaction + Send + Sync>,
    pub signature: Vec<u8>,

    #[serde(skip)]
    pub digest_hash: Vec<u8>,

    #[serde(skip)]
    pub bytes: Vec<u8>,

    #[serde(skip)]
    pub id: ids::Id,

    #[serde(skip)]
    pub size: u64,

    #[serde(skip)]
    pub sender: Address,
}

impl Transaction {
    pub fn new(
        unsigned_transaction: Box<dyn super::unsigned::Transaction + Send + Sync>,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            unsigned_transaction,
            signature,
            digest_hash: vec![],
            bytes: vec![],
            id: ids::Id::empty(),
            size: 0,
            sender: Address::zero(),
        }
    }
}

#[typetag::serde]
#[tonic::async_trait]
impl crate::chain::tx::Transaction for Transaction {
    async fn init(&mut self) -> Result<()> {
        let stx =
            serde_json::to_vec(&self).map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let typed_data = &self.unsigned_transaction.typed_data().await;
        let digest_hash = decoder::hash_structured_data(typed_data)?;

        let sender = key::secp256k1::public_key::Key::from_signature(
            digest_hash.as_bytes(),
            &self.signature,
        )?;
        self.bytes = stx;
        self.id = ids::Id::from_slice(hash::keccak256(&self.bytes).as_bytes());
        self.size = self.bytes.len() as u64;
        self.digest_hash = digest_hash.as_bytes().to_vec();
        self.sender = sender.to_h160();

        Ok(())
    }

    async fn bytes(&self) -> &Vec<u8> {
        return &self.bytes;
    }

    async fn size(&self) -> u64 {
        return self.size;
    }

    async fn id(&self) -> ids::Id {
        return self.id;
    }

    async fn execute(
        &self,
        db: &'life1 Box<dyn subnet::rpc::database::Database + Send + Sync>,
        block: &Block,
    ) -> Result<()> {
        log::debug!("execute: sender: {}", self.sender);
        let txn_ctx = TransactionContext {
            db: db.clone(),
            tx_id: self.id,
            block_time: block.timestamp,
            sender: self.sender,
        };

        self.unsigned_transaction
            .execute(txn_ctx)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        log::debug!("execute: set tx");
        set_transaction(db.clone(), self.to_owned())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        log::debug!("execute complete");
        Ok(())
    }
}

pub fn new_tx(
    utx: Box<dyn super::unsigned::Transaction + Send + Sync>,
    signature: Vec<u8>,
) -> Transaction {
    Transaction {
        unsigned_transaction: utx,
        signature,

        // defaults
        digest_hash: vec![],
        bytes: vec![],
        id: ids::Id::empty(),
        size: 0,
        sender: Address::zero(),
    }
}
