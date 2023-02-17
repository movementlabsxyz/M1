use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::ids;
use serde::{Deserialize, Serialize};

use crate::chain::{
    storage::{has_space, put_space_info},
    tx::decoder::{create_typed_data, MessageValue, Type, TypedData},
};

use super::{
    base,
    decoder::{TD_BLOCK_ID, TD_SPACE, TD_STRING},
    tx::TransactionType,
    unsigned,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Info {
    pub created: u64,
    pub updated: u64,

    #[serde(deserialize_with = "ids::short::must_deserialize_id")]
    pub raw_space: ids::short::Id,

    pub owner: ethereum_types::H160,
}

/// Creates a space, which acts as a logical key-space root.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tx {
    pub base_tx: base::Tx,
    pub space: String,
}

// important to define an unique name of the trait implementation
#[typetag::serde(name = "claim")]
#[tonic::async_trait]
impl unsigned::Transaction for Tx {
    async fn get_block_id(&self) -> avalanche_types::ids::Id {
        self.base_tx.block_id
    }

    async fn set_block_id(&mut self, id: avalanche_types::ids::Id) {
        self.base_tx.block_id = id;
    }

    async fn get_value(&self) -> Option<Vec<u8>> {
        None
    }

    async fn set_value(&mut self, _value: Vec<u8>) -> std::io::Result<()> {
        Err(Error::new(
            ErrorKind::Unsupported,
            "value is not supported for delete tx",
        ))
    }

    async fn typ(&self) -> TransactionType {
        TransactionType::Set
    }

    async fn execute(&self, txn_ctx: unsigned::TransactionContext) -> Result<()> {
        let mut db = txn_ctx.db;
        // TODO: ensure expected format of space

        // ensure space does not exist for now update requires an explicit delete tx
        if has_space(&db, self.space.as_bytes()).await? {
            log::debug!("execute: space exists: {}", self.space);
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("space exists: {}", self.space),
            ));
        }
        log::debug!("execute: space exec sender: {}", &txn_ctx.sender);
        let new_info = Info {
            created: txn_ctx.block_time,
            updated: txn_ctx.block_time,
            owner: txn_ctx.sender,
            raw_space: ids::short::Id::empty(),
        };

        return put_space_info(&mut db, self.space.as_bytes(), new_info, 0).await;
    }

    async fn typed_data(&self) -> TypedData {
        let mut tx_fields: Vec<Type> = Vec::new();
        tx_fields.push(Type {
            name: TD_SPACE.to_owned(),
            type_: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_BLOCK_ID.to_owned(),
            type_: TD_STRING.to_owned(),
        });

        let mut message: HashMap<String, MessageValue> = HashMap::with_capacity(1);
        message.insert(
            TD_SPACE.to_owned(),
            MessageValue::Vec(self.space.as_bytes().to_vec()),
        );
        let value = MessageValue::Vec(self.base_tx.block_id.to_vec());
        log::debug!("typed_data: message value: {:?}", value);
        log::debug!("typed_data: id vec: {:?}", self.base_tx.block_id.to_vec());
        log::debug!("typed_data: id: {}", self.base_tx.block_id);
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Claim, tx_fields, message);
    }
}
