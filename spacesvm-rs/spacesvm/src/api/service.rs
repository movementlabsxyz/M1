use std::sync::Arc;

use crate::{
    api::*,
    chain::{self, storage, tx::Transaction},
    vm::inner::Inner,
};

use tokio::sync::RwLock;

pub struct Service {
    pub vm_inner: Arc<RwLock<Inner>>,
}

impl Service {
    pub fn new(vm_inner: Arc<RwLock<Inner>>) -> Self {
        Self { vm_inner }
    }
}

impl crate::api::Service for Service {
    /// Returns true if the API is serving requests.
    fn ping(&self) -> BoxFuture<Result<PingResponse>> {
        log::debug!("ping called");

        Box::pin(async move { Ok(PingResponse { success: true }) })
    }

    /// Takes tx args and returns the tx id.
    fn issue_tx(&self, params: IssueTxArgs) -> BoxFuture<Result<IssueTxResponse>> {
        log::debug!("issue tx called");
        let vm = Arc::clone(&self.vm_inner);

        Box::pin(async move {
            let mut inner = vm.write().await;

            let unsigned_tx = params
                .typed_data
                .parse_typed_data()
                .map_err(create_jsonrpc_error)?;

            let mut tx = chain::tx::tx::Transaction::new(unsigned_tx, params.signature);
            tx.init().await.map_err(create_jsonrpc_error)?;
            let tx_id = tx.id().await;

            let mut txs = Vec::with_capacity(1);
            txs.push(tx);

            storage::submit(&inner.state, &mut txs).await.map_err(|e| {
                create_jsonrpc_error(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                ))
            })?;

            let mempool = &mut inner.mempool;
            for tx in txs.iter().cloned() {
                let _ = mempool.add(&tx).map_err(|e| {
                    create_jsonrpc_error(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;
                log::debug!("issue_tx add to mempool");
            }

            Ok(IssueTxResponse { tx_id })
        })
    }

    fn decode_tx(&self, params: DecodeTxArgs) -> BoxFuture<Result<DecodeTxResponse>> {
        log::debug!("decode input called");
        let vm = Arc::clone(&self.vm_inner);

        Box::pin(async move {
            let mut utx = params.tx_data.decode().map_err(create_jsonrpc_error)?;
            let inner = vm.write().await;
            let last_accepted = &inner
                .state
                .get_last_accepted()
                .await
                .map_err(create_jsonrpc_error)?;

            utx.set_block_id(*last_accepted).await;
            let typed_data = utx.typed_data().await;

            let string = serde_json::to_string(&typed_data).unwrap();

            log::debug!("decode_tx: {}", string);

            Ok(DecodeTxResponse { typed_data })
        })
    }

    fn resolve(&self, params: ResolveArgs) -> BoxFuture<Result<ResolveResponse>> {
        log::debug!("resolve: called");
        let vm = Arc::clone(&self.vm_inner);

        Box::pin(async move {
            let inner = vm.read().await;
            let db = inner.state.get_db().await;
            let value = chain::storage::get_value(&db, &params.space, &params.key)
                .await
                .map_err(create_jsonrpc_error)?;
            if value.is_none() {
                return Ok(ResolveResponse::default());
            }

            let meta = chain::storage::get_value_meta(&db, &params.space, &params.key)
                .await
                .map_err(create_jsonrpc_error)?;
            if meta.is_none() {
                return Ok(ResolveResponse::default());
            }

            Ok(ResolveResponse {
                exists: true,
                value: value.unwrap(),
                meta: meta.unwrap(),
            })
        })
    }
}
