use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use serde::{Deserialize, Serialize};

use crate::chain::{
    storage,
    tx::decoder::{create_typed_data, MessageValue, Type, TypedData},
};

use super::{
    base,
    decoder::{TD_BLOCK_ID, TD_KEY, TD_SPACE, TD_STRING},
    tx::TransactionType,
    unsigned,
};

/// Removes a key and value from the underlying space. No error will return
/// if the key is not found.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Tx {
    pub base_tx: base::Tx,
    pub space: String,
    pub key: String,
}

// important to define an unique name of the trait implementation
#[typetag::serde(name = "delete")]
#[tonic::async_trait]
impl unsigned::Transaction for Tx {
    async fn get_block_id(&self) -> avalanche_types::ids::Id {
        self.base_tx.block_id
    }

    async fn set_block_id(&mut self, id: avalanche_types::ids::Id) {
        self.base_tx.block_id = id
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
        TransactionType::Delete
    }

    async fn execute(&self, mut txn_ctx: unsigned::TransactionContext) -> Result<()> {
        let db = txn_ctx.db.clone();

        let info = storage::get_space_info(&db, self.space.as_bytes())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        if info.is_none() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("space not found: {}", self.space),
            ));
        }
        let info = info.unwrap();
        if info.owner != txn_ctx.sender {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!("sets only allowed for space owner: {}", self.space),
            ));
        }

        // while we do not use value meta currently we verify it exists.
        let v = storage::get_value_meta(&db, self.space.as_bytes(), self.key.as_bytes())
            .await
            .map_err(|e| {
                Error::new(ErrorKind::Other, format!("failed to get value meta: {}", e))
            })?;

        if v.is_none() {
            return Err(Error::new(ErrorKind::Other, "key is missing"));
        }

        storage::delete_space_key(&mut txn_ctx.db, self.space.as_bytes(), self.key.as_bytes())
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
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
        tx_fields.push(Type {
            name: TD_KEY.to_owned(),
            type_: TD_STRING.to_owned(),
        });

        let mut message: HashMap<String, MessageValue> = HashMap::with_capacity(1);
        message.insert(
            TD_SPACE.to_owned(),
            MessageValue::Vec(self.space.as_bytes().to_vec()),
        );
        message.insert(
            TD_KEY.to_owned(),
            MessageValue::Vec(self.key.as_bytes().to_vec()),
        );
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Delete, tx_fields, message);
    }
}
