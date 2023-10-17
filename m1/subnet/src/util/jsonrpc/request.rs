use std::io;
use bytes::Bytes;
use jsonrpc_core::MethodCall;

pub fn de_request(req: &Bytes) -> io::Result<String> {
    let method_call: MethodCall = serde_json::from_slice(req).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to deserialize request: {e}"),
        )
    })?;
    serde_json::to_string(&method_call).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("failed to serialize request: {e}"),
        )
    })
}