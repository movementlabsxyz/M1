use std::{
    fmt::Debug,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{ids::Id, subnet};
use dyn_clone::DynClone;
use serde::{Deserialize, Serialize};

use crate::chain::tx::decoder::TypedData;

use super::{base, claim, delete, set, tx::TransactionType};

#[typetag::serde(tag = "type")]
#[tonic::async_trait]
pub trait Transaction: Debug + DynClone + Send + Sync {
    async fn get_block_id(&self) -> Id;
    async fn set_block_id(&mut self, id: Id);
    async fn get_value(&self) -> Option<Vec<u8>>;
    async fn set_value(&mut self, value: Vec<u8>) -> Result<()>;
    async fn execute(&self, txn_ctx: TransactionContext) -> Result<()>;
    async fn typed_data(&self) -> TypedData;
    async fn typ(&self) -> TransactionType;
}

// ref. https://docs.rs/dyn-clone/latest/dyn_clone/macro.clone_trait_object.html
dyn_clone::clone_trait_object!(Transaction);

pub struct TransactionContext {
    pub db: Box<dyn subnet::rpc::database::Database + Send + Sync>,
    pub block_time: u64,
    pub tx_id: Id,
    pub sender: ethereum_types::Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TransactionData {
    pub typ: TransactionType,
    pub space: String,
    pub key: String,
    pub value: Vec<u8>,
}

impl TransactionData {
    pub fn decode(&self) -> Result<Box<dyn Transaction + Send + Sync>> {
        let tx_param = self.clone();
        match tx_param.typ {
            TransactionType::Claim => Ok(Box::new(claim::Tx {
                base_tx: base::Tx::default(),
                space: tx_param.space,
            })),
            TransactionType::Set => Ok(Box::new(set::Tx {
                base_tx: base::Tx::default(),
                space: tx_param.space,
                key: tx_param.key,
                value: tx_param.value,
            })),
            TransactionType::Delete => Ok(Box::new(delete::Tx {
                base_tx: base::Tx::default(),
                space: tx_param.space,
                key: tx_param.key,
            })),
            TransactionType::Unknown => Err(Error::new(
                ErrorKind::Other,
                "transaction type Unknown is not valid",
            )),
        }
    }
}
