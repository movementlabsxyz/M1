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