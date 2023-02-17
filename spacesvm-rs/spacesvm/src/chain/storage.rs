use std::{
    io::{Error, ErrorKind, Result},
    str,
};

use avalanche_types::{ids, subnet};
use byteorder::{BigEndian, ByteOrder};
use chrono::Utc;

use serde::{Deserialize, Serialize};

use crate::{
    block::{
        state::{self, HASH_LEN},
        Block,
    },
    chain::crypto,
};

use super::tx::{self, claim, Transaction};

const SHORT_ID_LEN: usize = 20;
const BLOCK_PREFIX: u8 = 0x0;
const TX_PREFIX: u8 = 0x1;
const TX_VALUE_PREFIX: u8 = 0x2;
const INFO_PREFIX: u8 = 0x3;
const KEY_PREFIX: u8 = 0x4;

pub const BYTE_DELIMITER: u8 = b'/';

pub async fn set_transaction(
    mut db: Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    tx: tx::tx::Transaction,
) -> Result<()> {
    let k = prefix_tx_key(&tx.id);
    return db.put(&k, &vec![]).await;
}

pub async fn delete_space_key(
    db: &mut Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
    key: &[u8],
) -> Result<()> {
    match get_space_info(db, space).await? {
        None => Err(Error::new(
            ErrorKind::InvalidData,
            format!("space not found"),
        )),
        Some(info) => {
            db.delete(&space_value_key(info.raw_space, key))
                .await
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
            Ok(())
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ValueMeta {
    pub size: u64,
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub tx_id: ids::Id,

    pub created: u64,
    pub updated: u64,
}

pub async fn submit(state: &state::State, txs: &mut Vec<tx::tx::Transaction>) -> Result<()> {
    let now = Utc::now().timestamp() as u64;
    let db = &state.get_db().await;

    for tx in txs.iter_mut() {
        tx.init()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
        if tx.id().await == ids::Id::empty() {
            return Err(Error::new(ErrorKind::Other, "invalid block id"));
        }
        let dummy_block = Block::new_dummy(now, tx.to_owned(), state.clone());

        tx.execute(&db, &dummy_block)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;
    }

    Ok(())
}

pub async fn get_value(
    db: &Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
    key: &[u8],
) -> Result<Option<Vec<u8>>> {
    let info: Option<tx::claim::Info> = match get_space_info(db, space).await {
        Ok(info) => info,
        Err(e) => {
            if is_not_found(&e) {
                return Ok(None);
            }
            return Err(e);
        }
    };
    if info.is_none() {
        return Ok(None);
    }

    let value = db
        .get(&space_value_key(info.unwrap().raw_space, key))
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    let vmeta: ValueMeta = serde_json::from_slice(&value)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let tx_id = vmeta.tx_id;

    log::error!("get_value tx_id: {:?}", tx_id);

    let value_key = prefix_tx_value_key(&tx_id);

    let value = db
        .get(&value_key)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

    Ok(Some(value))
}

pub async fn get_value_meta(
    db: &Box<dyn avalanche_types::subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
    key: &[u8],
) -> Result<Option<ValueMeta>> {
    match get_space_info(&db, space).await? {
        None => Ok(None),
        Some(info) => match db.get(&space_value_key(info.raw_space, key)).await {
            Err(e) => {
                if is_not_found(&e) {
                    return Ok(None);
                }
                Err(e)
            }
            Ok(value) => {
                let vmeta: ValueMeta = serde_json::from_slice(&value)
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                Ok(Some(vmeta))
            }
        },
    }
}

// Attempts to write the value
pub async fn put_space_key(
    db: &mut Box<dyn subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
    key: &[u8],
    vmeta: ValueMeta,
) -> Result<()> {
    let resp = get_space_info(db, space)
        .await
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
    if resp.is_none() {
        return Err(Error::new(ErrorKind::NotFound, format!("space not found")));
    }

    let k = space_value_key(resp.unwrap().raw_space, key);
    log::info!("put_value key: {:?}", k);
    let rv_meta = serde_json::to_vec(&vmeta)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    return db.put(&k, &rv_meta).await;
}

/// Attempts to store the space info by using a key 'space_info_key' with the value
/// being serialized space info.
pub async fn put_space_info(
    db: &mut Box<dyn subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
    mut info: claim::Info,
    _last_expiry: u64,
) -> Result<()> {
    // If [raw_space] is empty, this is a new space.
    if info.raw_space.is_empty() {
        log::info!("put_space_info: new space found");
        let r_space = raw_space(space, info.created)
            .await
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        log::info!("put_space_info: raw_space: {:?}", r_space);
        info.raw_space = r_space;
    }
    let value =
        serde_json::to_vec(&info).map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

    let key = &space_info_key(space);
    log::info!("put_space_info key: {:?}", key);
    log::info!("put_space_info value: {:?}", value);

    db.put(key, &value).await
}

// Attempts to get info from a space.
pub async fn get_space_info(
    db: &Box<dyn subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
) -> Result<Option<claim::Info>> {
    match db.get(&space_info_key(space)).await {
        Err(e) => {
            if is_not_found(&e) {
                return Ok(None);
            }
            Err(e)
        }
        Ok(value) => {
            let info: claim::Info = serde_json::from_slice(&value)
                .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

            log::info!("get_space_info info: {:?}", info);
            Ok(Some(info))
        }
    }
}

pub async fn raw_space(space: &[u8], block_time: u64) -> Result<ids::short::Id> {
    let mut r: Vec<u8> = Vec::new();
    r.extend_from_slice(space);
    r.push(BYTE_DELIMITER);
    r.resize(space.len() + 1 + 8, 20);
    BigEndian::write_u64(&mut r[space.len() + 1..].to_vec(), block_time);
    let hash = crypto::compute_hash_160(&r);

    Ok(ids::short::Id::from_slice(&hash))
}

/// Returns true if a space with the same name already exists.
pub async fn has_space(
    db: &Box<dyn subnet::rpc::database::Database + Send + Sync>,
    space: &[u8],
) -> Result<bool> {
    db.has(&space_info_key(space)).await
}

/// 'KEY_PREFIX' + 'BYTE_DELIMITER' + [r_space] + 'BYTE_DELIMITER' + [key]
pub fn space_value_key(r_space: ids::short::Id, key: &[u8]) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(2 + SHORT_ID_LEN + 1 + key.len());
    k.push(KEY_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(r_space.as_ref());
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(key);
    k
}

/// 'INFO_PREFIX' + 'BYTE_DELIMITER' + [space]
pub fn space_info_key(space: &[u8]) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(space.len() + 2);
    k.push(INFO_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(space);
    k
}

/// 'BLOCK_PREFIX' + 'BYTE_DELIMITER' + 'block_id'
pub fn prefix_block_key(block_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(BLOCK_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(&block_id.to_vec());
    k
}

/// 'TX_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
pub fn prefix_tx_key(tx_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(TX_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(&tx_id.to_vec());
    k
}

/// 'TX_VALUE_PREFIX' + 'BYTE_DELIMITER' + 'tx_id'
pub fn prefix_tx_value_key(tx_id: &ids::Id) -> Vec<u8> {
    let mut k: Vec<u8> = Vec::with_capacity(HASH_LEN);
    k.push(TX_VALUE_PREFIX);
    k.push(BYTE_DELIMITER);
    k.extend_from_slice(&tx_id.to_vec());
    k
}

/// Returns false if the io::Error is ErrorKind::NotFound and contains a string "not found".
pub fn is_not_found(error: &Error) -> bool {
    if error.kind() == ErrorKind::NotFound && error.to_string().contains("not found") {
        return true;
    }
    false
}

#[test]
fn test_prefix() {
    // 'KEY_PREFIX' [4] + 'BYTE_DELIMITER' [47] + [raw_space] 0 x 20 + 'BYTE_DELIMITER' [4] + [key] [102, 111, 111]
    assert_eq!(
        space_value_key(ids::short::Id::empty(), "foo".as_bytes().to_vec().as_ref()),
        [4, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 47, 102, 111, 111]
    );
    // 'INFO_PREFIX' [3] + 'BYTE_DELIMITER' [47] + 'space' [102, 111, 111]
    assert_eq!(
        space_info_key("foo".as_bytes().to_vec().as_ref()),
        [3, 47, 102, 111, 111]
    );
    // 'BLOCK_PREFIX' [0] + 'BYTE_DELIMITER' [47] + 'block_id' 0 x 32
    assert_eq!(
        prefix_block_key(&ids::Id::empty()),
        [
            0, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0
        ]
    );
    // 'TX_PREFIX' [1] + 'BYTE_DELIMITER' [47] + 'tx_id' 0 x 32
    assert_eq!(
        prefix_tx_key(&ids::Id::empty()),
        [
            1, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0
        ]
    );
    // 'TX_VALUE_PREFIX' [2] + 'BYTE_DELIMITER' [47] + 'tx_id' 0 x 32
    assert_eq!(
        prefix_tx_value_key(&ids::Id::empty()),
        [
            2, 47, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0
        ]
    )
}

#[tokio::test]
async fn test_raw_space() {
    let resp = raw_space("kvs".as_bytes(), 0).await;
    assert!(resp.is_ok());
    assert_eq!(
        resp.unwrap(),
        ids::short::Id::from_slice(&[
            230, 185, 125, 2, 27, 125, 127, 228, 212, 79, 188, 214, 107, 248, 146, 237, 254, 112,
            153, 17
        ])
    )
}

#[tokio::test]
async fn test_space_info_rt() {
    use super::tx::claim::Info;
    use ethereum_types::H160;

    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    let space = "kvs".as_bytes();
    let new_info = Info {
        created: 0,
        updated: 1,
        owner: H160::default(),
        raw_space: ids::short::Id::empty(),
    };
    let mut db = subnet::rpc::database::memdb::Database::new();
    // put
    let resp = put_space_info(&mut db, &space, new_info, 2).await;
    assert!(resp.is_ok());

    // get
    let resp = get_space_info(&mut db, &space).await;
    assert!(resp.as_ref().is_ok());
    assert!(resp.as_ref().unwrap().is_some());
    let info = resp.unwrap().unwrap();
    assert_eq!(
        info.raw_space,
        ids::short::Id::from_slice(&[
            230, 185, 125, 2, 27, 125, 127, 228, 212, 79, 188, 214, 107, 248, 146, 237, 254, 112,
            153, 17
        ])
    );
    assert_eq!(info.updated, 1);
}
