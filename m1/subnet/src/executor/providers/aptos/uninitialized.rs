use super::super::super::executor::Uninitialized as UninitializedExecutor;
use super::initialized::Initialized;
use super::aptos::AptosState;
use aptos_api::{Context, get_raw_api_service};
use aptos_config::config::NodeConfig;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_mempool::core_mempool::CoreMempool;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::ChainId;
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::{Transaction, WriteSetPayload};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use aptos_vm_genesis::{test_genesis_change_set_and_validators};
use tonic::async_trait;
use std::fs;
use chrono::{DateTime, Utc};
use futures::{channel::mpsc as futures_mpsc, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const MOVE_DB_DIR: &str = ".move-chain-data";

#[derive(Debug, Clone)]
pub struct Uninitialized;

impl AptosState for Uninitialized {}

impl Uninitialized {
    pub fn new() -> Self {
        Uninitialized {}
    }
}

impl Default for Uninitialized {
    fn default() -> Self {
        Uninitialized::new()
    }
}

#[async_trait]
impl UninitializedExecutor for Uninitialized {
    type Initialized = Initialized;
    type Config = NodeConfig;

    async fn initialize(self, config : Self::Config) -> Result<Self::Initialized, anyhow::Error> {
        
         // generate the test genesis
         let (genesis, validators) = test_genesis_change_set_and_validators(Some(1));
         let signer = ValidatorSigner::new(
             validators[0].data.owner_address,
             validators[0].consensus_key.clone(),
         );
         let signer = signer.clone();
 
         // write the genesis transaction
         let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
         let p = format!("{}/{}",
                         dirs::home_dir().unwrap().to_str().unwrap(),
                         MOVE_DB_DIR);
         if !fs::metadata(p.clone().as_str()).is_ok() {
             fs::create_dir_all(p.as_str()).unwrap();
         }
 
         // initialize aptos db
        let (_, db_reader_writer) = DbReaderWriter::wrap(
             AptosDB::new_for_test(p.as_str())
        );

        // waypoint
        // todo: check that requiring waypoint to receive is not a breaking change
        let waypoint = generate_waypoint::<AptosVM>(&db_reader_writer, &genesis_txn)?;
        maybe_bootstrap::<AptosVM>(&db_reader_writer, &genesis_txn, waypoint).unwrap();
         
         // BLOCK-STM
         // AptosVM::set_concurrency_level_once(2);
         let db = Arc::new(RwLock::new(db_reader_writer.clone()));
         let executor =  Arc::new(RwLock::new(BlockExecutor::new(db_reader_writer.clone())));
 

         // set up the mempool
         let (mempool_client_sender,
             mut mempool_client_receiver) = futures_mpsc::channel::<MempoolClientRequest>(10);
         let sender = MempoolClientSender::from(mempool_client_sender);
         let context = Context::new(ChainId::test(),
                                    db_reader_writer.reader.clone(),
                                    sender, config.clone());
 
         // initialze the raw api
        let api_context = context.clone();
        let api_service = get_raw_api_service(Arc::new(context));
        let core_mempool = Arc::new(RwLock::new(CoreMempool::new(&config)));

        // todo: implement_check_pending_tx
        // self.check_pending_tx().await;
 
        // todo: move into mempool operations
        // start the mempool client
        tokio::task::spawn(async move {
            while let Some(request) = mempool_client_receiver.next().await {
                match request {
                    MempoolClientRequest::SubmitTransaction(_t, callback) => {
                        // accept all the transaction
                        let ms = MempoolStatus::new(MempoolStatusCode::Accepted);
                        let status: SubmissionStatus = (ms, None);
                        callback.send(
                            Ok(status)
                        ).unwrap();
                    }
                    MempoolClientRequest::GetTransactionByHash(_, _) => {}
                }
            }
        });

        Ok(Initialized::new(
            api_service,
            api_context,
            core_mempool,
            db,
            signer,
            executor
        ))

    }
    
}
