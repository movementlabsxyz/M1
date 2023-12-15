use axum::{
    body::Body,
    extract::{Path, Query, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use serde_json::{json, Value};

use crate::subnet::*;
use crate::types::*;
use crate::utils::*;

pub async fn transactions(pagination: Option<Query<Pagination>>) -> impl IntoResponse {
    let Query(pagination) = pagination.unwrap_or_default();
    println!("transactions = {:?}", pagination);

    let json_data = request_to_subnet_ext::<Pagination>("getTransactions", Some(pagination))
        .await
        .unwrap();
    make_response(&json_data)
}

pub async fn submit_transactions(request: Request) -> impl IntoResponse {
    let (_parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();

    println!("submit_transactions");

    let json_data = request_to_subnet_ext::<serde_json::Value>(
        "submitTransaction",
        Some(json!({
            "data": bytes.clone()
        })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}

pub async fn submit_transaction_batch(request: Request) -> impl IntoResponse {
    let (_parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();

    println!("submit_transaction_batch");

    let json_data = request_to_subnet_ext::<serde_json::Value>(
        "submitTransactionBatch",
        Some(json!({
            "data": bytes.clone()
        })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}

pub async fn simulate_transaction(request: Request) -> impl IntoResponse {
    let (_parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();

    println!("simulate_transaction");

    let json_data = request_to_subnet_ext::<serde_json::Value>(
        "simulateTransaction",
        Some(json!({
            "data": bytes.clone()
        })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}

pub async fn get_transaction_by_hash(Path(txn_hash): Path<String>) -> impl IntoResponse {
    let param = if txn_hash.starts_with("0x") {
        txn_hash.strip_prefix("0x").unwrap().to_string()
    } else {
        txn_hash
    };

    println!("get_transaction_by_hash");
    let json_data =
        request_to_subnet_ext::<Value>("getTransactionByHash", Some(json!({ "data": param })))
            .await
            .unwrap();
    make_response(&json_data)
}

pub async fn get_transaction_by_version(Path(txn_version): Path<String>) -> impl IntoResponse {
    println!("get_transaction_by_version");
    let json_data = request_to_subnet_ext::<Value>(
        "getTransactionByVersion",
        Some(json!({ "version": txn_version })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}
