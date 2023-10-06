use aptos_api::{Context, get_raw_api_service, RawApi};
use aptos_api::accept_type::AcceptType;
use aptos_api::response::{AptosResponseContent, BasicResponse};
use aptos_api::transactions::{SubmitTransactionPost, SubmitTransactionResponse, SubmitTransactionsBatchPost, SubmitTransactionsBatchResponse};
use aptos_api_types::{Address, EncodeSubmissionRequest, IdentifierWrapper, MoveStructTag, RawTableItemRequest, StateKeyWrapper, TableItemRequest, ViewRequest};
use aptos_config::config::NodeConfig;
use aptos_crypto::{HashValue, ValidCryptoMaterialStringExt};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::BlockExecutorTrait;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_mempool::core_mempool::{CoreMempool, TimelineState};
use aptos_sdk::rest_client::aptos_api_types::MAX_RECURSIVE_TYPES_ALLOWED;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_storage_interface::DbReaderWriter;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::aptos_test_root_address;
use aptos_types::account_view::AccountView;
use aptos_types::block_info::BlockInfo;
use aptos_types::block_metadata::BlockMetadata;
use aptos_types::chain_id::ChainId;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo};
use aptos_types::mempool_status::{MempoolStatus, MempoolStatusCode};
use aptos_types::transaction::{SignedTransaction, Transaction, WriteSetPayload};
use aptos_types::transaction::Transaction::UserTransaction;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::AptosVM;
use aptos_vm_genesis::{GENESIS_KEYPAIR, test_genesis_change_set_and_validators};
use tonic::async_trait;

#[async_trait]
pub trait PreAptosOperations {

    async fn initialize_aptos(&self, node_config: NodeConfig) -> Result<Self, String>;

}

#[async_trait]
pub trait InitializedAptosOperations {
    
}

#[derive(Debug, Clone)]
pub struct Pre {

}

impl Default for Pre {
    fn default() -> Self {
        Pre {}
    }
}

#[derive(Debug, Clone)]
pub struct Initialized {

    pub api_service: RawApi,

    pub api_context: Context,

    pub core_mempool: Arc<RwLock<CoreMempool>>,

    pub db: Arc<RwLock<DbReaderWriter>>,

    pub signer: ValidatorSigner,

    pub executor: Arc<RwLock<BlockExecutor<AptosVM, Transaction>>>,

}

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

}

#[async_trait]
impl PreAptosOperations for Aptos<Pre> {
    
}