use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Result},
};

use avalanche_types::{hash, ids};
use eip_712::Type as ParserType;
use ethereum_types::H256;
use serde::{de, Deserialize, Serialize};
use serde_json::to_value;

use super::{base, claim, delete, set, tx::TransactionType, unsigned};

pub const TD_STRING: &str = "string";
pub const TD_U64: &str = "u64";
pub const TD_BYTES: &str = "bytes";
pub const TD_BLOCK_ID: &str = "blockId";
pub const TD_SPACE: &str = "space";
pub const TD_KEY: &str = "key";
pub const TD_VALUE: &str = "value";

pub type Type = eip_712::FieldType;

pub type Types = HashMap<String, Vec<Type>>;

pub type TypedDataMessage = HashMap<String, MessageValue>;

// TypedDataDomain represents the domain part of an EIP-712 message.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct TypedDataDomain {
    pub name: String,
    pub magic: String,
}

pub fn mini_kvvm_domain(_m: u64) -> TypedDataDomain {
    TypedDataDomain {
        name: "SpacesVm".to_string(),
        magic: "0x00".to_string(), // radix(m, 10).to_string(),
    }
}

#[derive(Debug, Clone)]
pub enum MessageValue {
    // TODO:combine?
    Vec(Vec<u8>),
    Bytes(Vec<u8>),
}

impl MessageValue {
    pub fn to_string(self) -> String {
        match self {
            MessageValue::Vec(v) => String::from_utf8_lossy(&v).to_string(),
            MessageValue::Bytes(v) => String::from_utf8_lossy(&v).to_string(),
        }
    }
    pub fn to_vec(self) -> Vec<u8> {
        match self {
            MessageValue::Vec(v) => v,
            MessageValue::Bytes(v) => v,
        }
    }
}

impl Serialize for MessageValue {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MessageValue::Vec(v) => serializer.serialize_str(&hex::encode(v)),
            MessageValue::Bytes(v) => {
                serializer.serialize_str(format!("0x{}", &hex::encode(v)).as_str())
            }
        }
    }
}

impl<'de> Deserialize<'de> for MessageValue {
    fn deserialize<D: de::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        struct MessageValueVisitor;
        impl<'de> de::Visitor<'de> for MessageValueVisitor {
            type Value = MessageValue;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a potential or array of potentials")
            }

            fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Self::Value, E> {
                if v.starts_with("0x") {
                    match hex::decode(&v[2..]) {
                        Ok(s) => Ok(MessageValue::Bytes(s)),
                        Err(e) => Err(E::custom(e.to_string())),
                    }
                } else {
                    match hex::decode(v) {
                        Ok(s) => Ok(MessageValue::Vec(s)),
                        Err(e) => Err(E::custom(e.to_string())),
                    }
                }
            }

            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Self::Value, E> {
                if v.starts_with("0x") {
                    match hex::decode(&v[2..]) {
                        Ok(s) => Ok(MessageValue::Bytes(s)),
                        Err(e) => Err(E::custom(e.to_string())),
                    }
                } else {
                    match hex::decode(v) {
                        Ok(s) => Ok(MessageValue::Vec(s)),
                        Err(e) => Err(E::custom(e.to_string())),
                    }
                }
            }
        }

        deserializer.deserialize_any(MessageValueVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TypedData {
    pub types: Types,
    pub primary_type: TransactionType,
    pub domain: TypedDataDomain,
    pub message: TypedDataMessage,
}

pub fn create_typed_data(
    tx_type: TransactionType,
    tx_fields: Vec<Type>,
    message: TypedDataMessage,
) -> TypedData {
    let mut types = Types::new();
    types.insert(tx_type.to_string(), tx_fields);
    types.insert(
        "EIP712Domain".to_owned(),
        vec![
            Type {
                name: "name".to_owned(),
                type_: "string".to_owned(),
            },
            Type {
                name: "magic".to_owned(),
                type_: "uint64".to_owned(),
            },
        ],
    );
    return TypedData {
        types,
        message,
        domain: mini_kvvm_domain(0), // TODO: pass magic
        primary_type: tx_type,
    };
}

impl TypedData {
    // Attempts to return the base tx from typed data.
    pub fn parse_base_tx(&self) -> Result<base::Tx> {
        let r_block_id = self
            .get_typed_message_vec(TD_BLOCK_ID.to_owned())
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;

        let block_id = ids::Id::from_slice(&r_block_id);

        Ok(base::Tx { block_id })
    }

    // Attempts to return and unsigned transaction from typed data.
    pub fn parse_typed_data(&self) -> Result<Box<dyn unsigned::Transaction + Send + Sync>> {
        let base_tx = self.parse_base_tx().map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("failed to parse base tx: {:?}", e),
            )
        })?;

        match self.primary_type {
            TransactionType::Claim => {
                let space = self
                    .get_typed_message(TD_SPACE.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                Ok(Box::new(claim::Tx { base_tx, space }))
            }

            TransactionType::Set => {
                let space = self
                    .get_typed_message(TD_SPACE.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                let key = self
                    .get_typed_message(TD_KEY.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                let value = self
                    .get_typed_message(TD_VALUE.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                Ok(Box::new(set::Tx {
                    base_tx,
                    space,
                    key: key,
                    value: value.as_bytes().to_vec(),
                }))
            }

            TransactionType::Delete => {
                let space = self
                    .get_typed_message(TD_SPACE.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                let key = self
                    .get_typed_message(TD_KEY.to_owned())
                    .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
                Ok(Box::new(delete::Tx {
                    base_tx,
                    space,
                    key: key,
                }))
            }
            TransactionType::Unknown => Err(Error::new(
                ErrorKind::Other,
                "transaction type Unknown is not valid",
            )),
        }
    }

    pub fn get_typed_message(&self, key: String) -> Result<String> {
        match self.message.get(&key) {
            Some(value) => Ok(value.to_owned().to_string()),
            None => Err(Error::new(
                ErrorKind::NotFound,
                format!("typed data key missing: {:?}", key),
            )),
        }
    }

    pub fn get_typed_message_vec(&self, key: String) -> Result<Vec<u8>> {
        match self.message.get(&key) {
            Some(value) => Ok(value.to_owned().to_vec()),
            None => Err(Error::new(
                ErrorKind::NotFound,
                format!("typed data key missing: {:?}", key),
            )),
        }
    }
}

pub fn hash_structured_data(typed_data: &TypedData) -> Result<H256> {
    // EIP-191 compliant
    let error_handling = |e: eip_712::Error| Error::new(ErrorKind::Other, e.to_string());
    let prefix = (b"\x19\x01").to_vec();
    let domain = to_value(&typed_data.domain).unwrap();
    let message = to_value(&typed_data.message).unwrap();
    let (domain_hash, data_hash) = (
        eip_712::encode_data(
            &ParserType::Custom("EIP712Domain".into()),
            &typed_data.types,
            &domain,
            None,
        )
        .map_err(error_handling)?,
        eip_712::encode_data(
            &ParserType::Custom(typed_data.primary_type.to_string()),
            &typed_data.types,
            &message,
            None,
        )
        .map_err(error_handling)?,
    );
    let concat = [&prefix[..], &domain_hash[..], &data_hash[..]].concat();
    Ok(hash::keccak256(concat))
}

#[tokio::test]
async fn signature_recovers() {
    use avalanche_types::key;

    let secret_key = key::secp256k1::private_key::Key::generate().unwrap();
    let public_key = secret_key.to_public_key();

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Claim,
        space: "kvs".to_string(),
        key: String::new(),
        value: vec![],
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let utx = resp.unwrap();
    let hash = hash_structured_data(&utx.typed_data().await).unwrap();

    let sig = secret_key.sign_digest(hash.as_bytes()).unwrap();
    let sender =
        key::secp256k1::public_key::Key::from_signature(hash.as_bytes(), &sig.to_bytes()).unwrap();
    assert_eq!(public_key.to_string(), sender.to_string());
    assert_eq!(public_key, sender,);

    let tx_data = crate::chain::tx::unsigned::TransactionData {
        typ: TransactionType::Set,
        space: "kvs".to_string(),
        key: "foo".to_string(),
        value: "bar".as_bytes().to_vec(),
    };
    let resp = tx_data.decode();
    assert!(resp.is_ok());
    let mut utx = resp.unwrap();
    utx.set_block_id(ids::Id::from_slice("duuuu".as_bytes()))
        .await;
    let hash = hash_structured_data(&utx.typed_data().await).unwrap();

    let sig = secret_key.sign_digest(hash.as_bytes()).unwrap();
    let hash = hash_structured_data(&utx.typed_data().await).unwrap();
    let sender =
        key::secp256k1::public_key::Key::from_signature(hash.as_bytes(), &sig.to_bytes()).unwrap();
    assert_eq!(public_key.to_string(), sender.to_string());
    assert_eq!(public_key, sender,);
}
