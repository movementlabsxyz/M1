use std::io::{Error, ErrorKind, Result};

use avalanche_types::ids;
use serde::{Deserialize, Serialize};

use super::unsigned;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct Tx {
    #[serde(deserialize_with = "ids::must_deserialize_id")]
    pub block_id: ids::Id,
}

impl Tx {
    pub async fn get_block_id(&self) -> avalanche_types::ids::Id {
        self.block_id
    }

    pub async fn set_block_id(&mut self, id: avalanche_types::ids::Id) {
        self.block_id = id;
    }

    pub async fn execute_base(&self, _txn_ctx: unsigned::TransactionContext) -> Result<()> {
        if self.block_id.is_empty() {
            return Err(Error::new(ErrorKind::Other, "invalid block id"));
        }
        Ok(())
    }
}
