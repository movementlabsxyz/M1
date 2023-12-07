use sui_types::{
    messages_checkpoint::VerifiedCheckpoint,
    transaction::{Transaction, VerifiedTransaction},
    executable_transaction::VerifiedExecutableTransaction
};

#[derive(Debug, Clone)]
pub struct SuiBlockExecutor {

}

pub async fn execute_block(&self, block : Vec<VerifiedExecutableTransaction>)-> Result<(), anyhow::Error> {

    // detect objects in transactions with something similar to: https://github.com/MystenLabs/sui/blob/3af3e0a353ff3ce699a6be9b0ddbf8c05b61892d/crates/sui-core/src/authority.rs#L961
    // assign intersecting transactions to run sequentially in block order
    // assign non-intersecting transactions to run in parallel

    todo!();

}