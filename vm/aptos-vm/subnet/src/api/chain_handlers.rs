use std::io;
use std::marker::PhantomData;
use std::str::FromStr;

use avalanche_types::ids;
use avalanche_types::proto::http::Element;
use avalanche_types::subnet::rpc::http::handle::Handle;
use bytes::Bytes;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, IoHandler, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use aptos_api_types::U64;

use crate::block::Block;
use crate::api::de_request;
use crate::vm::Vm;

#[rpc]
pub trait Rpc {
    #[rpc(name = "submitTransaction", alias("aptosvm.submitTransaction"))]
    fn submit_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>>;

    #[rpc(name = "simulateTransaction", alias("aptosvm.simulateTransaction"))]
    fn simulate_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>>;

    #[rpc(name = "estimateGasPrice", alias("aptosvm.estimateGasPrice"))]
    fn estimate_gas_price(&self) -> BoxFuture<Result<SubmitTransactionRes>>;

    #[rpc(name = "faucet", alias("aptosvm.faucet"))]
    fn facet_apt(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>>;

    #[rpc(name = "createAccount", alias("aptosvm.createAccount"))]
    fn create_account(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>>;

    #[rpc(name = "lastAccepted", alias("aptosvm.lastAccepted"))]
    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>>;

    #[rpc(name = "getBlock", alias("aptosvm.getBlock"))]
    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>>;

    #[rpc(name = "getTransactionByHash", alias("aptosvm.getTransactionByHash"))]
    fn get_transaction_by_hash(&self, args: GetBlockArgs) -> BoxFuture<Result<GetTransactionRes>>;

    #[rpc(name = "getTransactionByVersion", alias("aptosvm.getTransactionByVersion"))]
    fn get_transaction_by_version(&self, args: GetTransactionByVersionArgs) -> BoxFuture<Result<GetTransactionRes>>;

    #[rpc(name = "viewFunction", alias("aptosvm.viewFunction"))]
    fn view_function(&self, args: ViewFunctionArgs) -> BoxFuture<Result<SubmitTransactionArgs>>;

    #[rpc(name = "getLedgerInfo", alias("aptosvm.getLedgerInfo"))]
    fn get_ledger_info(&self) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountResources", alias("aptosvm.getAccountResources"))]
    fn get_account_resources(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccount", alias("aptosvm.getAccount"))]
    fn get_account(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountModules", alias("aptosvm.getAccountModules"))]
    fn get_account_modules(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountResourcesState", alias("aptosvm.getAccountResourcesState"))]
    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getAccountsTransactions", alias("aptosvm.getAccountsTransactions"))]
    fn get_accounts_transactions(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "getTransactions", alias("aptosvm.getTransactions"))]
    fn get_transactions(&self, args: PageArgs) -> BoxFuture<Result<ViewFunctionArgs>>;

    #[rpc(name = "GetBlockByHeight", alias("aptosvm.GetBlockByHeight"))]
    fn get_block_by_height(&self, args: GetBlockByHeightArgs) -> BoxFuture<Result<ViewFunctionArgs>>;
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ViewFunctionArgs {
    pub data: String,
}


#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitTransactionArgs {
    ///  bsc payload as hex
    pub data: String,
}

#[serde_as]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitTransactionRes {
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LastAcceptedResponse {
    pub id: ids::Id,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockArgs {
    pub id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockByHeightArgs {
    pub height: u64,
    pub with_transactions: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTransactionByVersionArgs {
    pub version: U64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetTransactionRes {
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GetBlockResponse {
    pub block: Block,
    pub data: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountArgs {
    pub account: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountStateArgs {
    pub account: String,
    pub resource: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PageArgs {
    pub start: Option<U64>,
    pub limit: Option<u16>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountNumberRes {
    pub data: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AccountStrRes {
    pub data: String,
}


#[derive(Clone)]
pub struct ChainService {
    pub vm: Vm,
}

impl ChainService {
    pub fn new(vm: Vm) -> Self {
        Self { vm }
    }
}


impl Rpc for ChainService {
    fn submit_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>> {
        log::debug!("submit_transaction called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let r = vm.submit_transaction(hex::decode(args.data).unwrap()).await;
            Ok(SubmitTransactionRes { data: r })
        })
    }

    fn simulate_transaction(&self, args: SubmitTransactionArgs) -> BoxFuture<Result<SubmitTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let data = hex::decode(args.data).unwrap();
            let ret = vm.simulate_transaction(data).await;
            Ok(SubmitTransactionRes { data: ret })
        })
    }

    fn estimate_gas_price(&self) -> BoxFuture<Result<SubmitTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.estimate_gas_price().await;
            Ok(SubmitTransactionRes { data: ret })
        })
    }

    fn facet_apt(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>> {
        log::debug!("facet_apt called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let acc = hex::decode(args.account).unwrap();
            let ret = vm.facet_apt(acc).await;
            Ok(AccountStrRes { data: ret })
        })
    }

    fn create_account(&self, args: AccountArgs) -> BoxFuture<Result<AccountStrRes>> {
        log::debug!("create_account called");
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.create_account(args.account.as_str()).await;
            Ok(AccountStrRes { data: ret })
        })
    }

    fn last_accepted(&self) -> BoxFuture<Result<LastAcceptedResponse>> {
        log::debug!("last accepted method called");
        let vm = self.vm.clone();

        Box::pin(async move {
            let vm_state = vm.state.read().await;
            if let Some(state) = &vm_state.state {
                let last_accepted = state
                    .get_last_accepted_block_id()
                    .await
                    .map_err(create_jsonrpc_error)?;

                return Ok(LastAcceptedResponse { id: last_accepted });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }

    fn get_block(&self, args: GetBlockArgs) -> BoxFuture<Result<GetBlockResponse>> {
        let blk_id = ids::Id::from_str(&args.id).unwrap();
        log::info!("get_block called for {}", blk_id);

        let vm = self.vm.clone();

        Box::pin(async move {
            let vm_state = vm.state.read().await;
            if let Some(state) = &vm_state.state {
                let block = state
                    .get_block(&blk_id)
                    .await
                    .map_err(create_jsonrpc_error)?;
                let data = String::from_utf8_lossy(&block.data()).to_string();
                return Ok(GetBlockResponse { block, data });
            }

            Err(Error {
                code: ErrorCode::InternalError,
                message: String::from("no state manager found"),
                data: None,
            })
        })
    }

    fn get_transaction_by_hash(&self, args: GetBlockArgs) -> BoxFuture<Result<GetTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let hash = args.id.as_str();
            log::info!("get hash by {}",hash.clone());
            let ret = vm.get_transaction_by_hash(hash).await;
            return Ok(GetTransactionRes { data: ret });
        })
    }

    fn get_transaction_by_version(&self, args: GetTransactionByVersionArgs) -> BoxFuture<Result<GetTransactionRes>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_transaction_by_version(args.version).await;
            return Ok(GetTransactionRes { data: ret });
        })
    }

    fn view_function(&self, args: ViewFunctionArgs) -> BoxFuture<Result<SubmitTransactionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            log::info!("view_function called {}",args.data.clone());
            let ret = vm.view_function(args.data.as_str()).await;
            return Ok(SubmitTransactionArgs { data: ret });
        })
    }

    fn get_ledger_info(&self) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_ledger_info().await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account_resources(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_resources(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account_modules(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_modules(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_account_resources_state(&self, args: AccountStateArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_account_resources_state(args.account.as_str(),
                                                     args.resource.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_accounts_transactions(&self, args: AccountArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_accounts_transactions(args.account.as_str()).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_transactions(&self, args: PageArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_transactions(args.start, args.limit).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }

    fn get_block_by_height(&self, args: GetBlockByHeightArgs) -> BoxFuture<Result<ViewFunctionArgs>> {
        let vm = self.vm.clone();
        Box::pin(async move {
            let ret = vm.get_block_by_height(args.height, args.with_transactions).await;
            return Ok(ViewFunctionArgs { data: ret });
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChainHandler<T> {
    pub handler: IoHandler,
    _marker: PhantomData<T>,
}

#[tonic::async_trait]
impl<T> Handle for ChainHandler<T>
    where
        T: Rpc + Send + Sync + Clone + 'static,
{
    async fn request(
        &self,
        req: &Bytes,
        _headers: &[Element],
    ) -> std::io::Result<(Bytes, Vec<Element>)> {
        match self.handler.handle_request(&de_request(req)?).await {
            Some(resp) => Ok((Bytes::from(resp), Vec::new())),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to handle request",
            )),
        }
    }
}

impl<T: Rpc> ChainHandler<T> {
    pub fn new(service: T) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(Rpc::to_delegate(service));
        Self {
            handler,
            _marker: PhantomData,
        }
    }
}


fn create_jsonrpc_error(e: std::io::Error) -> Error {
    let mut error = Error::new(ErrorCode::InternalError);
    error.message = format!("{}", e);
    error
}
