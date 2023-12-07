use aptos_types::transaction::{Transaction as AptosTransaction};
use sui_types::transaction::{Transaction as SuiTransaction};

#[derive(Clone, Debug)]
pub enum Transaction {
    Aptos(AptosTransaction),
    Sui(SuiTransaction)
}

impl Transaction {



    pub fn is_aptos(&self) -> bool {
        match self {
            Transaction::Aptos(_) => true,
            Transaction::Sui(_) => false
        }
    }

    pub fn is_sui(&self) -> bool {
        match self {
            Transaction::Aptos(_) => false,
            Transaction::Sui(_) => true
        }
    }

}
