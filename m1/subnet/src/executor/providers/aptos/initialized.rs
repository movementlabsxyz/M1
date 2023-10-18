use super::super::super::executor::Initialized as InitializedExecutor;
use super::aptos::AptosState;
use aptos_api::{Context, RawApi, transactions::TransactionsApi};
use aptos_crypto::HashValue;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor_types::{BlockExecutorTrait, StateComputeResult};
use aptos_mempool::core_mempool::CoreMempool;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::account_address::AccountAddress;
use aptos_types::block_info::BlockInfo;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo};
use aptos_types::transaction::Transaction;
use aptos_types::transaction::Transaction::UserTransaction;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use serde_with::rust::sets_last_value_wins;
use tonic::async_trait;
use tokio::sync::RwLock;
use std::sync::Arc;
use crate::util::types::{
    aptos::AptosBlock,
    block::Block,
};


#[derive(Clone)]
pub struct Initialized {

    pub api_service: RawApi,

    pub api_context: Context,

    pub core_mempool: Arc<RwLock<CoreMempool>>,

    pub db: Arc<RwLock<DbReaderWriter>>,

    pub signer: ValidatorSigner,

    pub executor: Arc<RwLock<BlockExecutor<AptosVM, Transaction>>>,

}

impl AptosState for Initialized {}

impl Initialized {

    pub fn new(
        api_service: RawApi,
        api_context: Context,
        core_mempool: Arc<RwLock<CoreMempool>>,
        db: Arc<RwLock<DbReaderWriter>>,
        signer: ValidatorSigner,
        executor: Arc<RwLock<BlockExecutor<AptosVM, Transaction>>>,
    ) -> Self {
        Initialized {
            api_service,
            api_context,
            core_mempool,
            db,
            signer,
            executor,
        }
    }

    pub fn get_pending_transactions(&self, count : u64) -> Result<Vec<Transaction>, anyhow::Error>  {

        let core_mempool = self.core_mempool.read().await;
        core_mempool.get_batch(count,
                            1024 * 5 * 1000,
                            true,
                            true, vec![])

    }

}


#[async_trait]
impl InitializedExecutor for Initialized {
    type ExecutionResult = (AptosBlock, StateComputeResult);
    type ExecutionResult = RawApi;

    pub async fn propose_block(
        &self, 
        parent_block_id : HashValue,
        height : u64,
    ) -> Result<Block, anyhow::Error> {

        // get the locks we need
        let transactions = self.get_pending_transactions(10000).await?;
        let executor = self.executor.write().await;
        let signer = self.signer;
        let db = self.db.read().await;

        // check the ledger info
        let last_ledger_info = db.reader.get_latest_ledger_info()?;
        let next_epoch = last_ledger_info.ledger_info().next_block_epoch();

        // build the block
        let block_id = HashValue::random();
        let block_meta = Transaction::BlockMetadata(BlockMetadata::new(
            block_id,
            next_epoch,
            0,
            signer.author(),
            vec![],
            vec![],
            unix_now,
        ));

        // metadata
        let mut block_transactions = vec![];
        block_transactions.push(block_meta);
        // transactions
        for tx in transactions.iter() {
            block_transactions.push(UserTransaction(tx.clone()));
        }
        // checkpoint
        block_transactions.push(Transaction::StateCheckpoint(HashValue::random()));

        // build the block
        // Avalanche and Aptos chain will have different block ids
        let aptos_parent_block_id = executor.committed_block_id();
        let aptos_block = AptosBlock::new(
            block_transactions,
            block_id.clone(),
            aptos_parent_block_id,
            next_epoch,
            unix_now,
        );

        let mut block = Block::new(
            parent_block_id,
            height,
            unix_now, // however they will be built at the same time
            serde_json::to_vec(&data)?,
            choices::status::Status::Processing,
        )?;

        Ok(block)
 
    }

    pub async fn execute_block(&self, block: Block) -> Result<Self::ExecutionResult, anyhow::Error> {

        let executor = self.executor.write().await;

        // get the aptos block and its metadata
        let aptos_block = AptosBlock::try_from(&block)?;
        let block_metadata = aptos_block.get_metadata()?;
 
        if aptos_block.block_id.ne(&block_metadata.id()) {
            return Err(Error::new(
                ErrorKind::Interrupted,
                "block format error",
            ));
        }

        let parent_block_should_be = executor.committed_block_id();
        if aptos_block.parent_block_id.ne(&parent_block_should_be) {
            return Err(Error::new(
                ErrorKind::Interrupted,
                "block format error",
            ));
        }
    
        // execute the block
        let result = executor.execute_block((block_id, block_tx.clone()), parent_block_id)?;

        Ok((aptos_block, result))
 
    }

    pub async fn commit_block(&self, (aptos_block, result) : Self::ExecutionResult) -> Result<(), anyhow::Error> {
        
        let signer = self.signer.as_ref().clone();
        let executor = self.executor.write().await;

        // sign for the the ledger
        let ledger_info = LedgerInfo::new(
            BlockInfo::new(
                aptos_block.next_epoch,
                0,
                aptos_block.block_id,
                result.root_hash(),
                result.version(),
                aptos_block.timestamp,
                result.epoch_state().clone(),
            ),
            HashValue::zero(),
        );

        let signed_ledger_info = generate_ledger_info_with_sig(
            &[signer], 
            ledger_info
        );

        // todo: the lock may need to be maintained from execution through commitment
        // todo: this can be implemented at a higher level and noted there
        // todo: for example, by passing the guard to the commit_block function
        let commitment_result = executor.commit_blocks(vec![
            aptos_block.block_id
        ], signed_ledger_info)?;

        // write transactions to the mempool
        {
            let mut core_pool = self.core_mempool.as_ref().write().await;
            for transaction in aptos_block.transactions.iter() {
                match transaction {
                    UserTransaction(transaction) => {
                        let sender = transaction.sender();
                        let sequence_number = transaction.sequence_number();
                        core_pool.commit_transaction(&AccountAddress::from(sender), sequence_number);
                    }
                    _ => {}
                }
            }
        }

        Ok(())

    }

    pub async fn get_api(&self) -> Result<Self::Api, anyhow::Error> {
        Ok(self.api_service.clone())
    }

    pub async fn get_transactions_api(&self) -> Result<TransactionsApi, anyhow::Error> {
        Ok(self.api_service.transactions_api)
    }

}
