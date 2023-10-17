use aptos_config::config::NodeConfig;
use avalanche_types::subnet::rpc::{snowman::block::CommonVm, consensus::snowman::Decidable};
use tonic::async_trait;
use crate::state::avalanche::avalanche_block::AvalancheBlock;
use super::super::{
    avalanche_aptos::{
        AvalancheAptos,
        AvalancheAptosVm,
        AvalancheAptosRuntime
    },
    initialized::Initialized,
    uninitialized::Uninitialized
};
use crate::util::types::{
    aptos::AptosBlock,
    block::Block
};
use avalanche_types::subnet::rpc::snow::validators::client::ValidatorStateClient;
use crate::rpc::static_service::{
    static_service::StaticService,
    avalanche_handler::StaticServiceAvalancheHandler
};
use crate::rpc::chain_service::{
    chain_service::ChainServiceRpc,
    avalanche_handler::ChainServiceAvalancheHandler
};
use avalanche_types::subnet::rpc::snow::engine::common::appsender::AppSender;
use avalanche_types::subnet::rpc::snow::engine::common::appsender::client::AppSenderClient;
use avalanche_types::subnet::rpc::database::manager::{DatabaseManager, Manager};
use avalanche_types::subnet::rpc::snow;

pub const VERSION : &str = "0.0.1";

// Bubble up to the generic
impl AvalancheAptos<Uninitialized> {

    pub async fn genesis(
        &self,
    )-> Result<(), anyhow::Error> {

        let state = self.state.state;

        let genesis = "hello world";
        let has_last_accepted = state.has_last_accepted_block().await?;
        if has_last_accepted {

            let last_accepted_block_id = state.get_last_accepted_block_id().await?;
            state.set_preferred(last_accepted_blk_id);

        } else {

            // construct the aptos genesis block
            let aptos_genesis_block = AptosBlock::genesis();

            // wrape the aptos genesis block into our block structure
            let mut genesis_block = Block::new(
                ids::Id::empty(),
                0,
                0,
                serde_json::to_vec(&aptos_genesis_block)?,
                choices::status::Status::default(),
            ).unwrap();

            // construct the avalanche block with a some state
            let mut avalanche_genesis_block = AvalancheBlock::new(
                genesis_block,
                state.clone()
            );

            // accept the genesis block in avalanche
            avalanche_genesis_block.accept().await?;

            // set the genesis block as the preferred block
            let genesis_block_id = genesis_block.id();
            state.set_preferred(&genesis_block_id);

        }
        
    }

    pub async fn initialize(&self) -> Result<AvalancheAptos<Initialized>, anyhow::Error> {
        
        let initialized_executor = {
            let executor = self.state.executor;
            executor.initialize(
                NodeConfig::default()
            ).await?
        };
    
        self.genesis().await?;

        let initialized = Initialized::new(
            initialized_executor,
            self.state.state.clone()
        );

        Ok(AvalancheAptos::new(initialized))

    }

}




#[async_trait]
impl CommonVm for AvalancheAptosVm {

    type DatabaseManager = DatabaseManager;
    type AppSender = AppSenderClient;
    type ChainHandler = ChainServiceAvalancheHandler</*todo*/>;
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

        match self.get_runtime().await? {
            AvalancheAptosRuntime::Uninitialized(uninitialized) => {
                // initialize
                let initialized = uninitialized.initialize().await?;

                // switch to the new runtime
                self.set_runtime(AvalancheAptosRuntime::Initialized(initialized));

                Ok(())

            },
            _ => Err(io::Error::new(io::ErrorKind::Other, "Already initialized")),
        }

    }

    async fn set_state(
        &self, 
        snow_state: snow::State
    ) -> io::Result<()> {
        
        self.set_snow_state(snow_state).await?;
        Ok(())

    }

    async fn version(&self) -> io::Result<String> {
        Ok(String::from(VERSION))
    }

    // todo: check that this is correct
    async fn create_static_handlers(
        &mut self,
    ) -> io::Result<HashMap<String, HttpHandler<Self::StaticHandler>>> {
        let handler = StaticHandler::new(StaticService::new());
        let mut handlers = HashMap::new();
        handlers.insert(
            "/static".to_string(),
            HttpHandler {
                lock_option: LockOptions::WriteLock,
                handler,
                server_addr: None,
            },
        );

        Ok(handlers)
    }

    // todo: check that this is correct
    async fn create_handlers(
        &mut self,
    ) -> io::Result<HashMap<String, HttpHandler<Self::ChainHandler>>> {
        let handler = ChainHandler::new(ChainService::new(self.clone()));
        let mut handlers = HashMap::new();
        handlers.insert(
            "/rpc".to_string(),
            HttpHandler {
                lock_option: LockOptions::WriteLock,
                handler,
                server_addr: None,
            },
        );

        Ok(handlers)
    }


}