#[async_trait]
pub trait SequencerLayer {

    type Transaction;
    type TransactionId;
    
    async fn send_transaction(
        &self,
        transaction: Self::Transaction
    ) -> Result<(), anyhow::Error>;


    async fn get_transaction(
        &self,
        transaction_id: Self::TransactionId
    ) -> Result<Self::Transaction, anyhow::Error>;

}