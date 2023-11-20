// todo: reduce import depth
use crate::transaction::transaction::Transaction;
use aptos_types::transaction::Transaction as AptosTransaction;

#[derive(Clone, Debug)]
pub struct Block(Vec<Transaction>);

impl Block {

    pub fn new(transactions: Vec<Transaction>) -> Self {
        Block(transactions)
    }

    /// Filters transactions to those enums which contain Aptos transactions.
    pub fn filter_aptos_transactions(&self) -> Vec<Transaction> {
        self.0.iter().filter(|t| t.is_aptos()).cloned().collect()
    }

    /// Extracts Aptos transactions refs from the block.
    pub fn extract_aptos_transaction_refs(&self) -> Vec<&AptosTransaction> {
        self.0.iter().filter_map(|transactions| {
            match transactions {
                Transaction::Aptos(aptos_transaction) => Some(aptos_transaction),
                _ => None
            }
        }).collect()
    }

    /// Extracts Aptos transactions from the block.
    pub fn extract_aptos_transactions(&self) -> Vec<AptosTransaction> {
        self.0.iter().filter_map(|transactions| {
            match transactions {
                Transaction::Aptos(aptos_transaction) => Some(aptos_transaction.clone()),
                _ => None
            }
        }).collect()
    }

}

impl Into<Vec<Transaction>> for Block {
    fn into(self) -> Vec<Transaction> {
        self.0
    }
}

impl Into<Block> for Vec<Transaction> {
    fn into(self) -> Block {
        Block(self)
    }
}