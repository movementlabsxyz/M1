//! Implementation of [`snowman.Block`](https://pkg.go.dev/github.com/ava-labs/avalanchego/snow/consensus/snowman#Block) interface for timestampvm.

use std::{
    fmt,
    io::{self, Error, ErrorKind},
};

use avalanche_types::{choices, ids};
use derivative::{self, Derivative};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// Represents a block, specific to [`Vm`](crate::vm::Vm).
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq)]
pub struct Block {
    /// The block Id of the parent block.
    parent_id: ids::Id,
    /// This block's height.
    /// The height of the genesis block is 0.
    height: u64,
    /// Unix second when this block was proposed.
    timestamp: u64,

    data: Vec<u8>,

    /// Current block status.
    #[serde(skip)]
    status: choices::status::Status,
    /// This block's encoded bytes.
    #[serde(skip)]
    bytes: Vec<u8>,
    /// Generated block Id.
    #[serde(skip)]
    id: ids::Id,
}

impl Block {
    pub fn new(
        parent_id: ids::Id,
        height: u64,
        timestamp: u64,
        data: Vec<u8>,
        status: choices::status::Status,
    ) -> io::Result<Self> {
        let mut b = Self {
            parent_id,
            height,
            timestamp,
            data,
            status: choices::status::Status::default(),
            bytes: Vec::new(),
            id: ids::Id::empty()
        };

        b.status = status;
        b.bytes = b.to_slice()?;
        b.id = ids::Id::sha256(&b.bytes);

        Ok(b)
    }

    pub fn to_json_string(&self) -> io::Result<String> {
        serde_json::to_string(&self).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to serialize Block to JSON string {}", e),
            )
        })
    }

    /// Encodes the [`Block`](Block) to JSON in bytes.
    pub fn to_slice(&self) -> io::Result<Vec<u8>> {
        serde_json::to_vec(&self).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to serialize Block to JSON bytes {}", e),
            )
        })
    }

    /// Loads [`Block`](Block) from JSON bytes.
    pub fn from_slice(d: impl AsRef<[u8]>) -> io::Result<Self> {
        let dd = d.as_ref();
        let mut b: Self = serde_json::from_slice(dd).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed to deserialize Block from JSON {}", e),
            )
        })?;

        b.bytes = dd.to_vec();
        b.id = ids::Id::sha256(&b.bytes);

        Ok(b)
    }

    /// Returns the parent block Id.
    pub fn parent_id(&self) -> ids::Id {
        self.parent_id
    }

    /// Returns the height of this block.
    pub fn height(&self) -> u64 {
        self.height
    }

    /// Returns the timestamp of this block.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// Returns the data of this block.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Returns the status of this block.
    pub fn status(&self) -> choices::status::Status {
        self.status.clone()
    }

    /// Updates the status of this block.
    pub fn set_status(&mut self, status: choices::status::Status) {
        self.status = status;
    }

    /// Returns the byte representation of this block.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the ID of this block
    pub fn id(&self) -> ids::Id {
        self.id
    }

}

/// ref. https://doc.rust-lang.org/std/string/trait.ToString.html
/// ref. https://doc.rust-lang.org/std/fmt/trait.Display.html
/// Use "Self.to_string()" to directly invoke this
impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let serialized = self.to_json_string().unwrap();
        write!(f, "{serialized}")
    }
}

