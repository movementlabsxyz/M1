use sui_types::{
    transaction::SenderSignedData,
    executable_transaction::VerifiedExecutableTransaction
};

/// A SuiBlock is a block as we would most often expect it to be constructed. 
/// It contains only user signed data. 
#[derive(Debug, Clone)]
pub struct Block(Vec<SenderSignedData>);

/// A VerifiedBlock is a block that has been verified by the SuiBlockExecutor; it contains `CertificateEnvelope`s for each transaction.
/// In most cases, this should be internally constructed.
#[derive(Debug, Clone)]
pub struct VerifiedExecutableBlock(Vec<VerifiedExecutableTransaction>);

#[derive(Debug, Clone)]
pub struct VerifiedExecutableExecutionGroups(pub Vec<Vec<VerifiedExecutableTransaction>>);

impl IntoIterator for VerifiedExecutableExecutionGroups {
    type Item = Vec<VerifiedExecutableTransaction>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl VerifiedExecutableBlock {

    pub fn new(transactions : Vec<VerifiedExecutableTransaction>) -> Self {
        Self(transactions)
    }

    pub fn get_max_parallel_groups(&self) -> VerifiedExecutableExecutionGroups {
        // todo: see readme
        unimplemented!();
    }


}

impl IntoIterator for VerifiedExecutableBlock {
    type Item = VerifiedExecutableTransaction;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}