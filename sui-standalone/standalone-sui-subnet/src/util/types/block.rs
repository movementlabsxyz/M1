use sui_types::transaction::{
    SenderSignedData,
    TransactionKind
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Block(SenderSignedData);

impl Into<SenderSignedData> for Block {
    fn into(self) -> SenderSignedData {
        self.0
    }
}

impl From<SenderSignedData> for Block {
    fn from(data: SenderSignedData) -> Self {
        Self(data)
    }
}