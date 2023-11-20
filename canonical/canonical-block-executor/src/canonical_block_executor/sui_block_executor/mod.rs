use std::sync::{
    Arc
};

use tokio::sync::{RwLock};

use sui_core::checkpoints::checkpoint_executor::{
    CheckpointExecutor
};

#[derive(Debug, Clone)]
pub struct SuiBlockExecutor {
    checkpoint_executor: Arc<RwLock<CheckpointExecutor>>
}

/*async fn init_executor_test(
    buffer_size: usize,
    store: Arc<CheckpointStore>,
) -> (
    Arc<AuthorityState>,
    CheckpointExecutor,
    Arc<StateAccumulator>,
    Sender<VerifiedCheckpoint>,
    CommitteeFixture,
) {
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
    (
        state,
        executor,
        accumulator,
        checkpoint_sender,
        CommitteeFixture::from_network_config(&network_config),
    )
}*/
