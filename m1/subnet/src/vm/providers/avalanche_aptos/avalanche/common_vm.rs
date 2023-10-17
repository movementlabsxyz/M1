use avalanche_types::subnet::rpc::snowman::block::CommonVm;
use tonic::async_trait;
use crate::state::avalanche::avalanche_block::AvalancheBlock;
use super::super::{
    avalanche_aptos::{
        AvalancheAptos,
        AvalancheAptosVm,
        AvalancheAptosRuntime
    },
    initialized::Initialized,
};
use crate::util::types::aptos::AptosData;

// Bubble up to the generic
impl AvalancheAptos<Uninitialized> {

    pub async fn genesis(
        &self,
    )-> Result<(), anyhow::Error> {

        let state = self.state;

        let genesis = "hello world";
        let has_last_accepted = state.has_last_accepted_block().await?;
        if has_last_accepted {
            let last_accepted_block_id = state.get_last_accepted_block_id().await?;
            state.set_preferred(last_accepted_blk_id);
        } else {
            let genesis_bytes = genesis.as_bytes().to_vec();
            let data = AptosData(
                genesis_bytes.clone(),
                HashValue::zero(),
                HashValue::zero(),
                0,
                0
            );
            let mut genesis_block = Block::new(
                ids::Id::empty(),
                0,
                0,
                serde_json::to_vec(&data).unwrap(),
                choices::status::Status::default(),
            ).unwrap();
            genesis_block.set_state(state.clone());
            genesis_block.accept().await?;

            let genesis_blk_id = genesis_block.id();
            state.set_preferred(genesis_blk_id);
        }
        
    }

    pub async fn initialize(&self) -> Result<(), anyhow::Error> {
        
        {
            let executor = self.executor;
            executor.initialize().await?;
        }
    
        self.genesis().await?;

        
    }

}




#[async_trait]
impl CommonVm for AvalancheAptosVm {

    type DatabaseManager = DatabaseManager;
    type AppSender = AppSenderClient;
    type ChainHandler = ChainHandler<ChainService>;
    type StaticHandler = StaticHandler;
    type ValidatorState = ValidatorStateClient;

    async fn initialize(
        &mut self,
        ctx: Option<subnet::rpc::context::Context<Self::ValidatorState>>,
        db_manager: Self::DatabaseManager,
        genesis_bytes: &[u8],
        _upgrade_bytes: &[u8],
        _config_bytes: &[u8],
        to_engine: Sender<snow::engine::common::message::Message>,
        _fxs: &[snow::engine::common::vm::Fx],
        app_sender: Self::AppSender,
    ) -> io::Result<()> {

        match self.get_runtime() {
            AvalancheAptosRuntime::Uninitialized(uninitialized) => {
                let initialized = uninitialized.initialize().await?;
                self.set_runtime(AvalancheAptosRuntime::Initialized(initialized));
                Ok(())
            },
            _ => Err(io::Error::new(io::ErrorKind::Other, "Already initialized")),
        }

    }


}