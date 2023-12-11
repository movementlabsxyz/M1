use aptos_types::transaction::Transaction;
use aptos_crypto::hash::HashValue;

#[derive(Debug, Clone)]
pub struct Block {
    pub transactions: Vec<Transaction>,
    pub block_id: HashValue,
    pub parent_block_id: HashValue,
    pub next_epoch : u64,
    pub timestamp : u64,
}