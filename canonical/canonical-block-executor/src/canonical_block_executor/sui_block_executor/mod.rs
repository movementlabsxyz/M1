use core::fmt;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use tokio::sync::RwLock;
use sui_core::{
    self, 
    transaction_manager::TransactionManager,
    checkpoints::{
        checkpoint_executor::{CheckpointExecutor, self},
        CheckpointStore
    },
    authority::{
        authority_per_epoch_store::AuthorityPerEpochStore,
        AuthorityStore,
        AuthorityState,
        test_authority_builder::TestAuthorityBuilder
    },
    state_accumulator::StateAccumulator,
};
use sui_types::{
    messages_checkpoint::VerifiedCheckpoint,
    transaction::{Transaction, VerifiedTransaction},
    executable_transaction::VerifiedExecutableTransaction
};
use tokio::sync::broadcast::{self, Sender, Receiver};
use sui_swarm_config::test_utils::CommitteeFixture;
use tempfile::tempdir;
use tracing_subscriber::field::debug;

#[derive(Clone)]
pub struct SuiBlockExecutor {
    pub authority_state : Arc<AuthorityState>,
    pub checkpoint_executor : Arc<RwLock<CheckpointExecutor>>,
    pub state_accumulator : Arc<StateAccumulator>,
    pub verified_checkpoint_sender : Sender<VerifiedCheckpoint>,
    pub committee_fixture : Arc<CommitteeFixture>,
}

impl fmt::Debug for SuiBlockExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuiBlockExecutor")
            .finish()
    }
}

impl SuiBlockExecutor {

    pub fn new(
        authority_state : Arc<AuthorityState>,
        checkpoint_executor : Arc<RwLock<CheckpointExecutor>>,
        state_accumulator : Arc<StateAccumulator>,
        verified_checkpoint_sender : Sender<VerifiedCheckpoint>,
        committee_fixture : Arc<CommitteeFixture>,
    ) -> Self {
        Self {
            authority_state,
            checkpoint_executor,
            state_accumulator,
            verified_checkpoint_sender,
            committee_fixture,
        }
    }

    pub async fn init(
        buffer_size : usize,
        path : Option<std::path::PathBuf>,
    )-> Result<Self, anyhow::Error> {

        let checkpoint_store_path = match path {
            Some(path_buf) => path_buf.as_path().to_owned(),
            None => {
                let dir = tempdir()?;
                dir.path().to_owned()
            },
        };

        let store = CheckpointStore::new(&checkpoint_store_path);
        
        let network_config =
        sui_swarm_config::network_config_builder::ConfigBuilder::new_with_temp_dir().build();
        let state = TestAuthorityBuilder::new()
            .with_network_config(&network_config)
            .build()
            .await;

        let (checkpoint_sender, _): (Sender<VerifiedCheckpoint>, Receiver<VerifiedCheckpoint>) =
            broadcast::channel(buffer_size);

        let accumulator = StateAccumulator::new(state.database.clone());
        let accumulator = Arc::new(accumulator);

        let executor = CheckpointExecutor::new_for_tests(
            checkpoint_sender.subscribe(),
            store.clone(),
            state.database.clone(),
            state.transaction_manager().clone(),
            accumulator.clone(),
        );

        Ok(Self::new(
            state,
            Arc::new(RwLock::new(executor)),
            accumulator,
            checkpoint_sender,
            Arc::new(CommitteeFixture::from_network_config(&network_config)),
        ))

    }

    pub async fn execute_block(&self, block : Vec<VerifiedExecutableTransaction>)-> Result<(), anyhow::Error> {

        let epoch_store = {
            let state = self.authority_state.clone();
        
            // Enqueue transactions in the transaction manager
            let transaction_manager: Arc<TransactionManager> = state.transaction_manager().clone();
            let epoch_store = state.epoch_store_for_testing().to_owned();
            debug!("Enqueueing transactions...");
            transaction_manager.enqueue(block, &epoch_store)?;  
            debug!("Enqueued transactions");
            epoch_store 
        };

        // Run the epoch
        // todo: confirm running the epoch waits for all transactions to be complete
        {
            let mut checkpoint_executor = self.checkpoint_executor.write().await;
            checkpoint_executor.run_epoch(epoch_store).await;
            
        }

        Ok(())

    }

}

#[cfg(test)]
mod test {

    use super::*;
    use tracing::{debug, error, info, warn};
    use tracing_subscriber::{FmtSubscriber, EnvFilter};

    use sui_test_transaction_builder::TestTransactionBuilder;

    use sui_types::{
        base_types::ObjectID,
        crypto::deterministic_random_account_key,
        digests::TransactionEffectsDigest,
        object::Object,
        storage::InputKey,
        transaction::{CallArg, ObjectArg},
        SUI_FRAMEWORK_PACKAGE_ID,
    };

    #[ctor::ctor]
    fn before_all() {
        // Create a filter based on the RUST_LOG environment variable
        let filter = EnvFilter::from_default_env();

        // Create a FmtSubscriber with the filter
        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .finish();

        // Set the subscriber as the global default
        tracing::subscriber::set_global_default(subscriber).expect("Failed to set the global tracing subscriber");
    }

    fn make_transaction(gas_object: Object, input: Vec<CallArg>) -> VerifiedExecutableTransaction {
        // Use fake module, function, package and gas prices since they are irrelevant for testing
        // transaction manager.
        let rgp = 100;
        let (sender, keypair) = deterministic_random_account_key();
        let transaction =
            TestTransactionBuilder::new(sender, gas_object.compute_object_reference(), rgp)
                .move_call(SUI_FRAMEWORK_PACKAGE_ID, "counter", "assert_value", input)
                .build_and_sign(&keypair);
        VerifiedExecutableTransaction::new_system(VerifiedTransaction::new_unchecked(transaction), 0)
    }

    #[tokio::test]
    async fn test_empty_block() {

        let executor = super::SuiBlockExecutor::init(10, None).await.unwrap();

        let block = vec![];

        executor.execute_block(block).await.unwrap();

    }

    #[tokio::test]
    async fn test_genesis_transaction() {
            
        let (owner, _keypair) = deterministic_random_account_key();

        let executor = super::SuiBlockExecutor::init(10, None).await.unwrap();

        let transaction = VerifiedExecutableTransaction::new_system(
            VerifiedTransaction::new_genesis_transaction(vec![]), 
            0
        );
        let block = vec![transaction];

        executor.execute_block(block).await.unwrap();
    }


    #[tokio::test]
    async fn test_single_transaction() {

        /*let (owner, _keypair) = deterministic_random_account_key();
        let gas_objects: Vec<Object> = (0..10)
            .map(|_| {
                let gas_object_id = ObjectID::random();
                Object::with_id_owner_for_testing(gas_object_id, owner)
            })
            .collect();

        let executor = super::SuiBlockExecutor::init(10, None).await.unwrap();

        let genesis_transaction = VerifiedExecutableTransaction::new_system(
            VerifiedTransaction::new_genesis_transaction(gas_objects.clone().), 
            0
        );
        let transaction = make_transaction(gas_objects[0].clone(), vec![]);
        let block = vec![transaction];

        executor.execute_block(block).await.unwrap();*/

    }

}