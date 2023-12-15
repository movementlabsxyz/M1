use axum::{
    body::Body,
    extract::{Path, Query, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::subnet::*;
use crate::types::*;
use crate::utils::*;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct TableParams {
    pub ledger_version: Option<String>,
}

pub async fn get_table_item(
    Path(table_handle): Path<String>,
    request: Request,
    params: Option<Query<TableParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let (_parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();

    println!("get_table_item");

    let json_data = request_to_subnet_ext::<serde_json::Value>(
        "getTableItem",
        Some(json!({
            "data": bytes.clone(),
            "query": table_handle,
            "ledge_version": params.ledger_version
        })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}

pub async fn get_raw_table_item(
    Path(table_handle): Path<String>,
    request: Request,
    params: Option<Query<TableParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let (_parts, body) = request.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes().to_vec();

    println!("get_raw_table_item");

    let json_data = request_to_subnet_ext::<serde_json::Value>(
        "getRawTableItem",
        Some(json!({
            "data": bytes.clone(),
            "query": table_handle,
            "ledge_version": params.ledger_version
        })),
    )
    .await
    .unwrap();
    make_response(&json_data)
}
