use std::{
    fs::File,
    io::{Error, ErrorKind, Result, Write},
    path::Path,
    sync::Arc,
};

use crate::{
    api::{
        DecodeTxArgs, DecodeTxResponse, IssueTxArgs, IssueTxResponse, PingResponse, ResolveArgs,
        ResolveResponse,
    },
    chain::tx::{
        decoder::{self, TypedData},
        tx::TransactionType,
        unsigned::TransactionData,
    },
};
use avalanche_types::key::{
    self,
    secp256k1::{private_key::Key, signature::Sig},
};
use http::{Method, Request};
use hyper::{body, client::HttpConnector, Body, Client as HyperClient};
use jsonrpc_core::{Call, Id, MethodCall, Params, Value, Version};
use serde::de;

pub use http::Uri;
use tokio::sync::RwLock;

/// Thread safe HTTP client for interacting with the API.
pub struct Client<C> {
    inner: Arc<RwLock<ClientInner<C>>>,
}

pub struct ClientInner<C> {
    id: u64,
    client: HyperClient<C>,
    endpoint: Uri,
    private_key: Option<Key>,
}

impl Client<HttpConnector> {
    pub fn new(endpoint: Uri) -> Self {
        let client = HyperClient::new();
        Self {
            inner: Arc::new(RwLock::new(ClientInner {
                id: 0,
                client,
                endpoint,
                private_key: None,
            })),
        }
    }
}

impl Client<HttpConnector> {
    async fn next_id(&self) -> Id {
        let mut client = self.inner.write().await;
        let id = client.id;
        client.id = id + 1;
        Id::Num(id)
    }

    pub async fn set_endpoint(&self, endpoint: Uri) {
        let mut inner = self.inner.write().await;
        inner.endpoint = endpoint;
    }

    pub async fn set_private_key(&self, private_key: Key) {
        let mut inner = self.inner.write().await;
        inner.private_key = Some(private_key);
    }

    /// Returns a serialized json request as string and the request id.
    pub async fn raw_request(&self, method: &str, params: &Params) -> Result<(Id, String)> {
        let id = self.next_id().await;
        let request = jsonrpc_core::Request::Single(Call::MethodCall(MethodCall {
            jsonrpc: Some(Version::V2),
            method: method.to_owned(),
            params: params.to_owned(),
            id: id.clone(),
        }));
        let request = serde_json::to_string(&request).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("jsonrpc request should be serializable: {}", e),
            )
        })?;

        Ok((id, request))
    }

    /// Returns a recoverable signature from 32 byte SHA256 message.
    pub async fn sign_digest(&self, dh: &[u8]) -> Result<Sig> {
        let inner = self.inner.read().await;
        if let Some(pk) = &inner.private_key {
            return pk.sign_digest(dh);
        }
        Err(Error::new(ErrorKind::Other, "private key not set"))
    }

    /// Returns a PingResponse from client request.
    pub async fn ping(&self) -> Result<PingResponse> {
        let (_id, json_request) = self.raw_request("ping", &Params::None).await?;
        let resp = self.post_de::<PingResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a DecodeTxResponse from client request.
    pub async fn decode_tx(&self, tx_data: TransactionData) -> Result<DecodeTxResponse> {
        let arg_value = serde_json::to_value(&DecodeTxArgs { tx_data })?;
        let (_id, json_request) = self
            .raw_request("decodeTx", &Params::Array(vec![arg_value]))
            .await?;
        let resp = self.post_de::<DecodeTxResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a IssueTxResponse from client request.
    pub async fn issue_tx(&self, typed_data: &TypedData) -> Result<IssueTxResponse> {
        let dh = decoder::hash_structured_data(typed_data)?;
        let sig = self.sign_digest(&dh.as_bytes()).await?.to_bytes().to_vec();
        log::debug!("signature: {:?}", sig);

        let arg_value = serde_json::to_value(&IssueTxArgs {
            typed_data: typed_data.to_owned(),
            signature: sig,
        })?;
        let (_id, json_request) = self
            .raw_request("issueTx", &Params::Array(vec![arg_value]))
            .await?;
        let resp = self.post_de::<IssueTxResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a ResolveResponse from client request.
    pub async fn resolve(&self, space: &str, key: &str) -> Result<ResolveResponse> {
        let arg_value = serde_json::to_value(&ResolveArgs {
            space: space.as_bytes().to_vec(),
            key: key.as_bytes().to_vec(),
        })?;
        let (_id, json_request) = self
            .raw_request("resolve", &Params::Array(vec![arg_value]))
            .await?;
        let resp = self.post_de::<ResolveResponse>(&json_request).await?;

        Ok(resp)
    }

    /// Returns a deserialized response from client request.
    pub async fn post_de<T: de::DeserializeOwned>(&self, json: &str) -> Result<T> {
        let inner = self.inner.read().await;

        let req = Request::builder()
            .method(Method::POST)
            .uri(inner.endpoint.to_string())
            .header("content-type", "application/json-rpc")
            .body(Body::from(json.to_owned()))
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("failed to create client request: {}", e),
                )
            })?;

        let mut resp = inner.client.request(req).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("client post request failed: {}", e),
            )
        })?;

        let bytes = body::to_bytes(resp.body_mut())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // deserialize bytes to value
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("failed to deserialize response to value: {}", e),
            )
        })?;

        // deserialize result to T
        let resp = serde_json::from_value(v["result"].to_owned()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("failed to deserialize response: {}", e),
            )
        })?;

        Ok(resp)
    }
}

pub fn claim_tx(space: &str) -> TransactionData {
    TransactionData {
        typ: TransactionType::Claim,
        space: space.to_owned(),
        key: String::new(),
        value: vec![],
    }
}

pub fn set_tx(space: &str, key: &str, value: &str) -> TransactionData {
    TransactionData {
        typ: TransactionType::Set,
        space: space.to_owned(),
        key: key.to_owned(),
        value: value.as_bytes().to_vec(),
    }
}

pub fn delete_tx(space: &str, key: &str) -> TransactionData {
    TransactionData {
        typ: TransactionType::Delete,
        space: space.to_owned(),
        key: key.to_owned(),
        value: vec![],
    }
}

/// Returns a private key from a given path or creates new.
pub fn get_or_create_pk(path: &str) -> Result<key::secp256k1::private_key::Key> {
    if !Path::new(path).try_exists()? {
        let secret_key = key::secp256k1::private_key::Key::generate().unwrap();
        let mut f = File::create(path)?;
        let hex = hex::encode(&secret_key.to_bytes());
        f.write_all(hex.as_bytes())?;
        return Ok(secret_key);
    }

    let contents = std::fs::read_to_string(path)?;
    let parsed = hex::decode(contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    key::secp256k1::private_key::Key::from_bytes(&parsed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}

#[tokio::test]
async fn test_raw_request() {
    let cli = Client::new(Uri::from_static("http://test.url"));
    let (id, _) = cli.raw_request("ping", &Params::None).await.unwrap();
    assert_eq!(id, jsonrpc_core::Id::Num(0));
    let (id, req) = cli.raw_request("ping", &Params::None).await.unwrap();
    assert_eq!(id, jsonrpc_core::Id::Num(1));
    assert_eq!(
        req,
        r#"{"jsonrpc":"2.0","method":"ping","params":null,"id":1}"#
    );
}
