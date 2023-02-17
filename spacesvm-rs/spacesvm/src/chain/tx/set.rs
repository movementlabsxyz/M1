use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
};

use serde::{Deserialize, Serialize};
use sha3::Digest;

use crate::chain::{
    storage::{self, get_space_info, put_space_info, put_space_key, ValueMeta},
    tx::decoder::{create_typed_data, MessageValue, Type, TypedData},
};

use super::{
    base,
    decoder::{TD_BLOCK_ID, TD_BYTES, TD_KEY, TD_SPACE, TD_STRING, TD_VALUE},
    tx::TransactionType,
    unsigned::{self},
};

/// 0x + hex-encoded hash
const HASH_LEN: usize = 66;

/// Performs a write against the logical keyspace. If the key exists
/// the value will be overwritten. The space must be created in
/// advance.
#[derive(Serialize, Deserialize, Clone, Debug)]

pub struct Tx {
    pub base_tx: base::Tx,

    /// Base namespace for the key value pair.
    pub space: String,

    /// Parsed from the given input, with its space removed.
    pub key: String,

    /// Written as the key-value pair to the storage. If a previous value
    /// exists, it is overwritten.
    pub value: Vec<u8>,
}

// important to define an unique name of the trait implementation
#[typetag::serde(name = "set")]
#[tonic::async_trait]
impl unsigned::Transaction for Tx {
    async fn get_block_id(&self) -> avalanche_types::ids::Id {
        self.base_tx.block_id
    }

    async fn set_block_id(&mut self, id: avalanche_types::ids::Id) {
        self.base_tx.block_id = id;
    }

    async fn get_value(&self) -> Option<Vec<u8>> {
        Some(self.value.clone())
    }

    async fn set_value(&mut self, value: Vec<u8>) -> std::io::Result<()> {
        self.value = value;
        Ok(())
    }

    async fn typ(&self) -> TransactionType {
        TransactionType::Set
    }

    async fn execute(&self, txn_ctx: unsigned::TransactionContext) -> std::io::Result<()> {
        let mut db = txn_ctx.db;
        // TODO: ensure expected format of space, key and value

        if self.key.len() == HASH_LEN {
            let hash = value_hash(&self.value);
            if self.key != hash {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("invalid key: {} expected: {}", self.key, hash),
                ));
            }
        }

        let value_size = self.value.len() as u64;

        let mut new_vmeta = ValueMeta {
            size: value_size,
            tx_id: txn_ctx.tx_id,
            created: txn_ctx.block_time,
            updated: txn_ctx.block_time,
        };

        let v = storage::get_value_meta(&db, self.space.as_bytes(), self.key.as_bytes()).await?;
        if v.is_none() {
            new_vmeta.created = txn_ctx.block_time;
        }

        let info = get_space_info(&db, self.space.as_bytes())
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
            log::debug!(
                "execute: owner: {}\n sender: {}",
                &info.owner,
                &txn_ctx.sender
            );
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!("sets only allowed for spaced owner: {}", self.space),
            ));
        }

        put_space_info(&mut db, self.space.as_bytes(), info, 0)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        log::debug!(
            "execute: put_space_key: space: {} key: {} value_meta: {:?}\n",
            self.space,
            self.key,
            new_vmeta
        );

        put_space_key(
            &mut db,
            self.space.as_bytes(),
            self.key.as_bytes(),
            new_vmeta,
        )
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    async fn typed_data(&self) -> TypedData {
        let mut tx_fields: Vec<Type> = vec![];
        tx_fields.push(Type {
            name: TD_SPACE.to_owned(),
            type_: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_KEY.to_owned(),
            type_: TD_STRING.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_VALUE.to_owned(),
            type_: TD_BYTES.to_owned(),
        });
        tx_fields.push(Type {
            name: TD_BLOCK_ID.to_owned(),
            type_: TD_STRING.to_owned(),
        });

        let mut message = HashMap::with_capacity(3);
        message.insert(
            TD_SPACE.to_owned(),
            MessageValue::Vec(self.space.as_bytes().to_vec()),
        );
        message.insert(
            TD_KEY.to_owned(),
            MessageValue::Vec(self.key.as_bytes().to_vec()),
        );
        message.insert(
            TD_VALUE.to_owned(),
            MessageValue::Bytes(self.value.to_vec()),
        );
        message.insert(
            TD_BLOCK_ID.to_owned(),
            MessageValue::Vec(self.base_tx.block_id.to_vec()),
        );

        return create_typed_data(super::tx::TransactionType::Set, tx_fields, message);
    }
}

fn value_hash(value: &[u8]) -> String {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(value);
    let result = hasher.finalize();
    hex::encode(&result[..])
}

#[tokio::test]
async fn set_tx_test() {
    use super::unsigned::Transaction;
    use std::str::FromStr;

    // set tx space not found
    let db = avalanche_types::subnet::rpc::database::memdb::Database::new();
    let ut_ctx = unsigned::TransactionContext {
        db,
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
        sender: ethereum_types::Address::zero(),
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.unwrap_err().kind() == ErrorKind::NotFound);

    // create space
    let db = avalanche_types::subnet::rpc::database::memdb::Database::new();
    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
        sender: ethereum_types::Address::zero(),
    };
    let tx = crate::chain::tx::claim::Tx {
        base_tx: base::Tx::default(),
        space: "kvs".to_string(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.is_ok());

    // try to update key from a different sender
    let other_account =
        ethereum_types::Address::from_str("0000000000000000000000000000000000000001").unwrap();
    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
        sender: other_account,
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert_eq!(resp.unwrap_err().kind(), ErrorKind::PermissionDenied);

    // try to update key from original sender
    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
        sender: ethereum_types::Address::zero(),
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.is_ok());

    let ut_ctx = unsigned::TransactionContext {
        db: db.clone(),
        block_time: 0,
        tx_id: avalanche_types::ids::Id::empty(),
        sender: ethereum_types::Address::zero(),
    };
    let tx = Tx {
        base_tx: base::Tx::default(),
        space: "kvs".to_string(),
        key: "bar".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx.execute(ut_ctx).await;
    assert!(resp.is_ok());
}
