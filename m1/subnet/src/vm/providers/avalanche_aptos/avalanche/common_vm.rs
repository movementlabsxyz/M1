use avalanche_types::subnet::rpc::snowman::block::CommonVm;
use super::super::avalanche_aptos::AvalancheAptosVm;
use tonic::async_trait;

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

        // initialize the vm
        self.initialize().await?;
        
        Ok(())
    }


}