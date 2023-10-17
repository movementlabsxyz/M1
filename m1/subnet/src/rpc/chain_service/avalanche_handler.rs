use super::chain_service::ChainServiceRpc;
use avalanche_types::proto::http::Element;
use avalanche_types::subnet::rpc::http::handle::Handle;
use bytes::Bytes;


#[derive(Clone, Debug)]
pub struct ChainServiceAvalancheHandler<T> {
    pub handler: IoHandler,
    _marker: PhantomData<T>,
}

#[tonic::async_trait]
impl<T> Handle for ChainServiceAvalancheHandler<T>
    where
        T: ChainServiceRpc + Send + Sync + Clone + 'static,
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

impl<T: ChainServiceRpc> ChainServiceAvalancheHandler<T> {
    pub fn new(service: T) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(ChainServiceRpc::to_delegate(service));
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
