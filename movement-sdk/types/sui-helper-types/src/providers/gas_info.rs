use sui_types::{
    transaction::{
        InputObjects,
        TransactionData
    },
    base_types::ObjectRef,
    gas::SuiGasStatus
};

#[async_trait::async_trait]
pub trait GasInforProvider {

    /// Provides up to date input objects for a transaction.
    /// Should be similar to this: https://github.com/MystenLabs/sui/blob/552158d9eae200314499809d8977f732f6c2cee7/crates/sui-transaction-checks/src/lib.rs#L50
    async fn gas_status(&self, transaction_data : &TransactionData, input_objects : &InputObjects, object_refs : &[ObjectRef]) -> Result<SuiGasStatus, anyhow::Error>;
    
}