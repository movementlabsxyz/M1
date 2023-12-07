// todo: reduce import depth
use crate::transaction::transaction::Transaction;
use aptos_types::transaction::Transaction as AptosTransaction;
use sui_types::transaction::SenderSignedData as SuiTransaction;
use aptos_crypto::hash::HashValue;
use aptos_helper_types::block::Block as AptosBlock;
use sui_helper_types::block::Block as SuiBlock;

#[derive(Clone, Debug)]
pub struct Block {
    pub transactions: Vec<Transaction>,
    pub aptos_block_id: HashValue,
    pub aptos_parent_block_id: HashValue,
    pub aptos_next_epoch : u64,
    pub aptos_timestamp : u64,
}

impl Block {

    pub fn new(
        transactions: Vec<Transaction>,
        aptos_block_id: HashValue,
        aptos_parent_block_id: HashValue,
        aptos_next_epoch : u64,
        aptos_timestamp : u64,
    ) -> Self {
        Block {
            transactions,
            aptos_block_id,
            aptos_parent_block_id,
            aptos_next_epoch,
            aptos_timestamp,
        }
    }

    /// Filters transactions to those enums which contain Aptos transactions.
    pub fn filter_aptos_transactions(&self) -> Vec<Transaction> {
        self.transactions.iter().filter(|t| t.is_aptos()).cloned().collect()
    }

    /// Extracts Aptos transactions refs from the block.
    pub fn get_aptos_transaction_refs(&self) -> Vec<&AptosTransaction> {
        self.transactions.iter().filter_map(|transaction| {
            match transaction {
                Transaction::Aptos(aptos_transaction) => Some(aptos_transaction),
                _ => None
            }
        }).collect()
    }

    /// Extracts Aptos transactions from the block.
    pub fn get_aptos_transactions(&self) -> Vec<AptosTransaction> {
        self.transactions.iter().filter_map(|transaction| {
            match transaction {
                Transaction::Aptos(aptos_transaction) => Some(aptos_transaction.clone()),
                _ => None
            }
        }).collect()
    }

    /// Gets an Aptos Block
    pub fn get_aptos_block(&self) -> AptosBlock {
        AptosBlock {
            transactions: self.get_aptos_transactions(),
            block_id: self.aptos_block_id,
            parent_block_id: self.aptos_parent_block_id,
            next_epoch: self.aptos_next_epoch,
            timestamp: self.aptos_timestamp,
        }
    }

    /// Gets a Sui Block
    pub fn get_sui_block(&self) -> SuiBlock {
        unimplemented!();
    }

}