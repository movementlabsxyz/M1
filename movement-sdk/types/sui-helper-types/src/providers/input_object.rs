use sui_types::transaction::{InputObjects, TransactionData};

#[async_trait::async_trait]
pub trait InputObjectProvider {

    /// Provides up to date input objects for a transaction.
    /// Should be similar to this: https://github.com/MystenLabs/sui/blob/f52a5db7c4e9343b3b522617bbbc064e9ef3b97f/crates/sui-core/src/transaction_input_loader.rs#L39
    async fn input_objects(&self, transaction_data : &TransactionData) -> Result<InputObjects, anyhow::Error>;

}