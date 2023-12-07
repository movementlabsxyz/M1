use crate::block::VerifiedExecutableExecutionGroups;

#[async_trait::async_trait]
pub trait ObjectVersionProvider {

    async fn sequence_objects_for_transactions(&self, transactions : VerifiedExecutableExecutionGroups) -> Result<VerifiedExecutableExecutionGroups, anyhow::Error>;

}