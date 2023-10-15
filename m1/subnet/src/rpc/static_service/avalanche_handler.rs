// Implements API services for the static handlers.
use super::static_service::{StaticService, PingResponse};
use avalanche_types::proto::http::Element;
use avalanche_types::subnet::rpc::http::handle::Handle;
use bytes::Bytes;


#[derive(Default)]
pub struct StaticServiceProvider {}

impl StaticServiceProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}



impl StaticService for StaticServiceProvider {
    fn ping(&self) -> BoxFuture<Result<PingResponse>> {
        log::debug!("ping called");
        Box::pin(async move { Ok(crate::api::PingResponse { success: true }) })
    }
}
#[derive(Clone)]
pub struct StaticServiceAvalancheHandler {
    pub handler: IoHandler,
}

impl StaticServiceAvalancheHandler {
    #[must_use]
    pub fn new(service: StaticServiceProvider) -> Self {
        let mut handler = jsonrpc_core::IoHandler::new();
        handler.extend_with(StaticService::to_delegate(service));
        Self { handler }
    }
}

#[tonic::async_trait]
impl Handle for StaticServiceAvalancheHandler {
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
