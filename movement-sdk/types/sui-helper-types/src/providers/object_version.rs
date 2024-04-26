use crate::block::VerifiedExecutableExecutionGroups;

// todo: expand this trait to include more analogs to these operations: https://github.com/MystenLabs/sui/blob/6ec723bcbdc4c36358d444cbfcd88ae1378761a5/crates/sui-core/src/authority/authority_per_epoch_store.rs#L301
#[async_trait::async_trait]
pub trait ObjectVersionProvider {

    /// Assigns sequence numbers to objects in the transactions
    async fn assign_shared_object_versions(&self, transactions : VerifiedExecutableExecutionGroups) -> Result<VerifiedExecutableExecutionGroups, anyhow::Error>;

}