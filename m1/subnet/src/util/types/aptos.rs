use serde::{Deserialize, Serialize};
use aptos_types::transaction::Transaction;
use aptos_types::block_metadata::BlockMetadata;
use aptos_crypto::HashValue;
use super::block::Block;

#[derive(Serialize, Deserialize, Clone)]
pub struct AptosBlock {
    pub transactions : Vec<Transaction>, // block info
    pub block_id : HashValue, // block id
    pub parent_block_id : HashValue,
    pub next_epoch : u64,
    pub timestamp : u64,
}

impl AptosBlock {

    pub fn new(
        transactions : Vec<Transaction>,
        block_id : HashValue,
        parent_block_id : HashValue,
        next_epoch : u64,
        timestamp : u64,
    ) -> Self {
        AptosData {
            transactions,
            block_id,
            parent_block_id,
            next_epoch,
            timestamp,
        }
    }

    pub fn genesis() -> Self {
        AptosData {
            transactions : vec![],
            block_id : HashValue::zero(),
            parent_block_id : HashValue::zero(),
            next_epoch : 0,
            timestamp : 0,
        }
    }

    pub fn get_metadata(&self) -> Result<BlockMetadata, anyhow::Error> {
        let transaction_0 = self.transactions.get(0)?;
        transaction_0.try_as_block_metadata()
    }

}

impl TryFrom<&Block> for AptosBlock {
    type Error = anyhow::Error;

    fn try_from(value: &Block) -> Result<Self, Self::Error> {
        serde_json::from_slice::<AptosData>(&value.data())
    }

}


#[derive(Serialize, Deserialize, Clone)]
pub struct AptosHeader {
    chain_id: u8,
    ledger_version: u64,
    ledger_oldest_version: u64,
    ledger_timestamp_usec: u64,
    epoch: u64,
    block_height: u64,
    oldest_block_height: u64,
    cursor: Option<String>,
}

impl AptosHeader {

    pub fn new(
        chain_id: u8,
        ledger_version: u64,
        ledger_oldest_version: u64,
        ledger_timestamp_usec: u64,
        epoch: u64,
        block_height: u64,
        oldest_block_height: u64,
        cursor: Option<String>,
    ) -> Self {
        AptosHeader {
            chain_id,
            ledger_version,
            ledger_oldest_version,
            ledger_timestamp_usec,
            epoch,
            block_height,
            oldest_block_height,
            cursor,
        }
    }

}
