use axum::{
    body::Body,
    extract::{Path, Query, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use serde_json::json;

use crate::subnet::*;
use crate::types::*;
use crate::utils::*;

pub async fn get_ledger_info() -> impl IntoResponse {
    println!("get_ledger_info");
    let json_data = request_to_subnet_ext::<String>("getLedgerInfo", None)
        .await
        .unwrap();
    make_response(&json_data)
}

pub async fn healthy() -> impl IntoResponse {
    (StatusCode::OK, "OK".to_string())
}

// TODO: implement this after checking `getAccount` api for balance check
pub async fn faucet() -> impl IntoResponse {
    StatusCode::OK
}

pub async fn view_request(request: Request) -> impl IntoResponse {
    let (_parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();

    println!("get_transaction_by_version");
    let json_data = request_to_subnet_ext::<serde_json::Value>(
        "viewFunction",
        Some(json!({
            "data": bytes.clone()
        })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}
