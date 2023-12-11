use sui_types::committee::EpochId;

#[async_trait::async_trait]
pub trait EpochProvider {

    /// Provides the current epoch id.
    async fn epoch_id(&self) -> Result<EpochId, anyhow::Error>;

    /// Provides the current epoch timestamp.
    async fn epoch_timestamp(&self) -> Result<u64, anyhow::Error>;
    

}