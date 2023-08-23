use std::str::FromStr;
use std::{io, any};
use std::marker::PhantomData;

use aptos_api::transactions::{
    SubmitTransactionPost
};
use aptos_types::account_address::AccountAddress;
use aptos_api_types::transaction::{
    UserTransactionRequest,
    UserTransactionRequestInner,
    TransactionSignature,
    EcdsaWMASignature,
    SubmitTransactionRequest,
    TransactionPayload,
};
use aptos_api_types::Address;
use aptos_sdk::transaction_builder::aptos_stdlib;
use avalanche_types::proto::http::Element;
use avalanche_types::subnet::rpc::http::handle::Handle;
use bytes::Bytes;
use jsonrpc_core::{BoxFuture, Error, ErrorCode, IoHandler, Result};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use aptos_api::accept_type::AcceptType;
use aptos_api_types::U64;

use crate::api::de_request;
use crate::vm::Vm;
use ethers::types::{TransactionRequest, Transaction, U256, U64 as EthU64, Signature, H160};
use serde_json;
use aptos_storage_interface::state_view::{LatestDbStateCheckpointView};
use aptos_state_view::account_with_state_view::{AccountWithStateView, AsAccountWithStateView};
use aptos_types::account_view::AccountView;
use aptos_api_types::{MoveResource};

#[derive(Debug, Serialize, Deserialize, Clone)]
enum QuantityOrTag {
    Tag(String),       // e.g. "latest", "earliest", "pending"
    Quantity(u64),     // integer block number
}



#[rpc]
pub trait EthRpc {
    

    /// GOSSIP METHODS 
    /// eth_blockNumber
    #[rpc(name = "eth_blockNumber")]
    fn eth_block_number(&self) -> BoxFuture<Result<String>>;

    /// eth_sendRawTransaction
    #[rpc(name = "eth_sendRawTransaction")]
    fn eth_send_raw_transaction(&self, data: String) -> BoxFuture<Result<String>>;

    // STATE METHODS:
    #[rpc(name = "eth_getBalance")]
    fn eth_get_balance(&self, address: String, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getStorageAt")]
    fn eth_get_storage_at(&self, address: String, position: String, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getTransactionCount")]
    fn eth_get_transaction_count(&self, address: String, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    /*
    #[rpc(name = "eth_getCode")]
    fn eth_get_code(&self, address: String, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_call")]
    fn eth_call(&self, transaction: TransactionRequest, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_estimateGas")]
    fn eth_estimate_gas(&self, transaction: TransactionRequest) -> BoxFuture<Result<String>>;

    // HISTORY METHODS:
    #[rpc(name = "eth_getBlockTransactionCountByHash")]
    fn eth_get_block_transaction_count_by_hash(&self, block_hash: String) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getBlockTransactionCountByNumber")]
    fn eth_get_block_transaction_count_by_number(&self, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getUncleCountByBlockHash")]
    fn eth_get_uncle_count_by_block_hash(&self, block_hash: String) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getUncleCountByBlockNumber")]
    fn eth_get_uncle_count_by_block_number(&self, block: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getBlockByHash")]
    fn eth_get_block_by_hash(&self, block_hash: String, full_transactions: bool) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getBlockByNumber")]
    fn eth_get_block_by_number(&self, block: QuantityOrTag, full_transactions: bool) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getTransactionByHash")]
    fn eth_get_transaction_by_hash(&self, tx_hash: String) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getTransactionByBlockHashAndIndex")]
    fn eth_get_transaction_by_block_hash_and_index(&self, block_hash: String, index: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getTransactionByBlockNumberAndIndex")]
    fn eth_get_transaction_by_block_number_and_index(&self, block: QuantityOrTag, index: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getTransactionReceipt")]
    fn eth_get_transaction_receipt(&self, tx_hash: String) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getUncleByBlockHashAndIndex")]
    fn eth_get_uncle_by_block_hash_and_index(&self, block_hash: String, index: QuantityOrTag) -> BoxFuture<Result<String>>;

    #[rpc(name = "eth_getUncleByBlockNumberAndIndex")]
    fn eth_get_uncle_by_block_number_and_index(&self, block: QuantityOrTag, index: QuantityOrTag) -> BoxFuture<Result<String>>;*/

  
}






#[derive(Clone)]
pub struct EthService {
    pub vm: Vm,
}

impl EthService {
    pub fn new(vm: Vm) -> Self {
        Self { vm }
    }
}

pub enum DetectedEthTransactionType {
    EtherTransfer,
    ContractDeployment,
    ContractInteraction,
    NoOp, // Represents an invalid or unrecognized transaction type
}

impl EthService {

    fn concatenate_rsv(r: U256, s: U256, v: EthU64) -> Vec<u8> {
       let sig = Signature {
        r, s, v : v.as_u64()
       };
       sig.to_vec()
    }
    

    fn h160_to_aptos(h160_address: &[u8; 20]) -> [u8; 32] {
        let mut aptos_address = [0u8; 32];  // Initialize with zeros
        aptos_address[12..].copy_from_slice(h160_address);  // Copy the H160 address to the end of the Aptos address
        aptos_address
    }

    fn detect_eth_transaction_type(tx: &Transaction) -> DetectedEthTransactionType {
        if tx.to.is_none() && !tx.input.is_empty() {
            DetectedEthTransactionType::ContractDeployment
        } else if tx.to.is_some() && !tx.input.is_empty() {
            DetectedEthTransactionType::ContractInteraction
        } else if tx.to.is_some() && tx.input.is_empty() {
            DetectedEthTransactionType::EtherTransfer
        } else {
            DetectedEthTransactionType::NoOp
        }
    }

    /*pub fn aptos_account_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::new([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 1,
                ]),
                ident_str!("aptos_account").to_owned(),
            ),
            ident_str!("transfer").to_owned(),
            vec![],
            vec![bcs::to_bytes(&to).unwrap(), bcs::to_bytes(&amount).unwrap()],
        ))
    }*/

    // Translates the ethereum transaction request to a submit transaction request
    pub fn eth_to_submit_transaction_request(&self, tx : Transaction) -> Result<Vec<u8>> {
        
        match Self::detect_eth_transaction_type(&tx) {
            DetectedEthTransactionType::EtherTransfer => {
                // form the transfer
                let transfer = aptos_stdlib::aptos_account_transfer(
                    AccountAddress::from_bytes(Self::h160_to_aptos(
                        &tx.to.expect("No recipient specified").to_fixed_bytes()
                    )).expect("Failed to convert recipient to AccountAddress"), 
                    0 // TODO: implement actual transfer value via multiple transactions
                    // ethereum transfer values are H256, but the aptos transfer is u64
                    // we would need to issue multiple transactions
                    // but for now, we will do without
                );

                // form the inner transaction request
                let user_transaction_request_inner = UserTransactionRequestInner {
                    sender: Address::from(
                        AccountAddress::from_bytes(Self::h160_to_aptos(&tx.from.to_fixed_bytes()))
                        .unwrap()
                    ),
                    sequence_number: aptos_api_types::U64(tx.nonce.as_u64()),
                    payload: serde_json::from_slice(
                        serde_json::to_string(&transfer).unwrap().as_bytes() // serialize, deserialize work around
                    ).unwrap(),
                    max_gas_amount: aptos_api_types::U64(tx.gas.as_u64()),
                    gas_unit_price: aptos_api_types::U64(tx.gas_price.expect("No gas price specified").as_u64()),
                    expiration_timestamp_secs: aptos_api_types::U64(0), // this is not being evaluated currently
                };

                // form the signature
                let transaction_signature = TransactionSignature::EcdsaWMASignature(
                    EcdsaWMASignature::new(
                        tx.rlp().to_vec(),
                        tx.from.as_fixed_bytes().to_vec(),
                        Self::concatenate_rsv(tx.r, tx.s, tx.v)
                    )
                );

                // form the transaction request
                let user_transaction_request = SubmitTransactionRequest {
                    user_transaction_request: user_transaction_request_inner,
                    signature: transaction_signature,
                };
                let user_transaction_request_bytes = bcs::to_bytes(&user_transaction_request).unwrap();

                Ok(aptos_api::bcs_payload::Bcs(user_transaction_request_bytes).to_vec())

            },
            DetectedEthTransactionType::ContractDeployment => {
                unimplemented!("Contract deployment detected.")
            },
            DetectedEthTransactionType::ContractInteraction => {
                unimplemented!("Contract interaction detected.")
            },
            DetectedEthTransactionType::NoOp => {
                unimplemented!("NoOp detected.")
            },
        }
    
    }

}

#[cfg(test)]
mod test_chain_service {

    use super::EthService;
    use ethers::types::{H160};
    use aptos_types::account_address::AccountAddress;
    
    #[test]
    pub fn test_h160_to_aptos() {

        let account = H160::random();
        let aptos_address = EthService::h160_to_aptos(account.as_fixed_bytes());
        AccountAddress::from_bytes(aptos_address).expect("Failed to convert H160 to Aptos address.");

    }

}


impl EthRpc for EthService {

    fn eth_block_number(&self) -> BoxFuture<Result<String>> {

        let vm = self.vm.clone();
        Box::pin(async move {
            let vm_state = vm.state.read().await;
            let state = vm_state.state
            .as_ref().expect("State not initialized.");

            // get the last accepeted block
            let id = state.get_last_accepted_block_id().await
            .expect("Failed to get last accepted block.");
            Ok(id.to_string())
        })
               
    }

    fn eth_send_raw_transaction(&self, data: String) -> BoxFuture<Result<String>> {

        let vm = self.vm.clone();
        let eth_self = self.clone();
        Box::pin(async move {
            let request = serde_json::from_str::<Transaction>(data.as_str()).unwrap();
            let submit = eth_self.eth_to_submit_transaction_request(request).unwrap();
            let res = vm.submit_transaction(
                submit, // broke serialization 
                AcceptType::Json
            ).await; 
            Ok(res.data.clone())            
        }) 

    }

    fn eth_get_balance(&self, account: String, block:  QuantityOrTag) -> BoxFuture<Result<String>> {

        // TODO: currently the block number is ignored and only the latest block is fetched

        let db = self.vm.db.as_ref().expect("DB not initialized.").clone();
        let account_h160 = H160::from_str(account.as_str()).unwrap();
        let account_address = AccountAddress::from_bytes(
            Self::h160_to_aptos(&account_h160.to_fixed_bytes())
        ).expect("Failed to convert H160 to Aptos address.");
        Box::pin(async move {
            let db_state_view = db.read().await.reader.latest_state_checkpoint_view().unwrap();
            let account_state_view = db_state_view.as_account_with_state_view(&account_address);
            let amount = account_state_view
                .get_coin_store_resource()
                .unwrap()
                .unwrap()
                .coin();
            Ok(amount.to_string())
        })

    }

    fn eth_get_storage_at(&self, account: String, position: String, block: QuantityOrTag) -> BoxFuture<Result<String>> {
        
    

        let db = self.vm.db.as_ref().expect("DB not initialized.").clone();

        let account_h160 = H160::from_str(account.as_str()).unwrap();
        let account_address = AccountAddress::from_bytes(
            Self::h160_to_aptos(&account_h160.to_fixed_bytes())
        ).expect("Failed to convert H160 to Aptos address.");

        Box::pin(async move {
            let db_state_view = db.read().await.reader.latest_state_checkpoint_view().unwrap();
            let account_state_view = db_state_view.as_account_with_state_view(&account_address);
            let resource = account_state_view
                .get_state_value(
                    &account_state_view.get_state_key_for_path(
                        position.into_bytes()
                    ).unwrap()
                ).unwrap().expect("Invalid position.");
            Ok(String::from_utf8(resource).unwrap())
        })

    }

    
    fn eth_get_transaction_count(&self, address: String, block: QuantityOrTag) -> BoxFuture<Result<String>> {
        // This will not be able work for applications expecting strict adherence to the ETH JSON-RPC spec
        unimplemented!("eth_get_transaction_count")
    }


}

#[cfg(test)]
mod test_eth_service {

    use avalanche_types::subnet::rpc::snow::engine::common::vm::CommonVm;

    use super::{EthService, EthRpc};
    use crate::vm::{Vm};
    
    #[tokio::test]
    pub async fn test_eth_block_number() -> Result<(), anyhow::Error> {

        let vm = Vm::new();
        // vm.initialize(ctx, db_manager, genesis_bytes, upgrade_bytes, config_bytes, to_engine, fxs, app_sender);
        let eth_service = EthService::new(vm.clone());
        let res = eth_service.eth_block_number().await;
        assert!(res.is_err());

        Ok(())

    }

}

#[derive(Clone, Debug)]
pub struct EthHandler<T> {
    pub handler: IoHandler,
    _marker: PhantomData<T>,
}

#[tonic::async_trait]
impl<T> Handle for EthHandler<T>
    where
        T: EthRpc + Send + Sync + Clone + 'static,
{
    async fn request(
        &self,
        req: &Bytes,
        _headers: &[Element],
    ) -> io::Result<(Bytes, Vec<Element>)> {
        match self.handler.handle_request(&de_request(req)?).await {
            Some(resp) => Ok((Bytes::from(resp), Vec::new())),
            None => Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to handle request",
            )),
        }
    }
}

impl<T: EthRpc> EthHandler<T> {
    pub fn new(service: T) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(EthRpc::to_delegate(service));
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
