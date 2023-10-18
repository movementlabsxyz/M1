use crate::rpc::chain_service::chain_service::{
    PageArgs,
    RpcRes,
    ChainServiceRpc,
    RpcReq, AccountStateArgs, RpcTableReq
};
use super::super::{
    avalanche_aptos::{
        AvalancheAptos,
        AvalancheAptosVm,
        AvalancheAptosRuntime
    },
    initialized::Initialized,
};
use jsonrpc_core::{BoxFuture, Result};



#[async_trait]
impl ChainServiceRpc for AvalancheAptos<Initialized> {

    
    // transactions
    fn get_transactions(&self, args: PageArgs) -> BoxFuture<Result<RpcRes>> {

        // get the executor
        let executor = self.state.executor;
        // todo: this needs to be non-await
        let transactions_api = executor.get_transactions_api().await?;

        Box::pin(async move {

            let accept = if args.is_bsc_format.unwrap_or(false) {
                AcceptType::Bcs
            } else {
                AcceptType::Json
            };
        
            let basic_response = transactions_api.get_transactions_raw(accept, args.start, args.limit).await?;
           
            basic_response.try_into()?

        })

    }

    fn get_transaction_by_hash(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_transaction_by_hash")
    }

    fn get_transaction_by_version(&self,args:crate::rpc::chain_service::chain_service::GetTransactionByVersionArgs) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_transaction_by_version")
    }

    fn simulate_transaction(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("simulate_transaction")
    }

    fn submit_transaction(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {

        let executor = self.state.executor;

        // todo: this needs to be non-await
        let transactions_api = executor.get_transactions_api().await?;

        unimplemented!("submit_transaction")
    }

    fn submit_transaction_batch(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("submit_transaction_batch")
    }

    // account
    fn create_account(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("create_account")
    }

    fn get_account(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_account")
    }

    fn get_account_modules(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_account_modules")
    }

    fn get_account_modules_state(&self, args: AccountStateArgs) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_account_modules_state")
    }

    fn estimate_gas_price(&self) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("estimate_gas_price")
    }

    fn get_account_resources(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_account_resources")
    }

    fn get_account_resources_state(&self,args:AccountStateArgs) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_account_resources_state")
    }

    fn get_accounts_transactions(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_accounts_transactions")
    }

    // blocks
    fn get_block_by_height(&self,args:crate::rpc::chain_service::chain_service::BlockArgs) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_block_by_height")
    }

    fn get_block_by_version(&self,args:crate::rpc::chain_service::chain_service::BlockArgs) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_block_by_version")
    }

    // events
    fn get_events_by_creation_number(&self,args:crate::rpc::chain_service::chain_service::RpcEventNumReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_events_by_creation_number")
    }

    fn get_events_by_event_handle(&self,args:crate::rpc::chain_service::chain_service::RpcEventHandleReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_events_by_event_handle")
    }

    // ledger info
    fn get_ledger_info(&self) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_ledger_info")
    }

    // table 
    fn get_raw_table_item(&self, args:  RpcTableReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_raw_table_item")
    }

    fn get_table_item(&self,args:RpcTableReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("get_table_item")
    }

    fn encode_submission(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("encode_submission")
    }

    fn faucet_apt(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("faucet_apt")
    }

    // view functions
    fn view_function(&self,args:RpcReq) -> BoxFuture<Result<RpcRes> > {
        unimplemented!("view_function")
    }

    // delegation
    fn to_delegate<M:jsonrpc_core::Metadata>(self) -> jsonrpc_core::IoDelegate<Self,M> {
        unimplemented!("to_delegate")
    }



}