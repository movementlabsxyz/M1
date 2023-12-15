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
pub struct BlockParams {
    pub with_transactions: Option<bool>,
}

pub async fn get_block_by_height(
    Path(height): Path<u64>,
    params: Option<Query<BlockParams>>,
) -> impl IntoResponse {
    
    let Query(params) = params.unwrap_or_default();
    let params = json!({
      "with_transactions": params.with_transactions.unwrap_or(false),
      "height_or_version": height
    });
    println!("get_block_by_height params ={}", params);

    let json_data = request_to_subnet_ext::<serde_json::Value>("getBlockByHeight", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_block_by_version(
    Path(version): Path<u64>,
    params: Option<Query<BlockParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let params = json!({
      "with_transactions": params.with_transactions.unwrap_or(false),
      "height_or_version": version
    });
    
    println!("get_block_by_version params ={}", params);

    let json_data = request_to_subnet_ext::<serde_json::Value>("getBlockByVersion", Some(params)).await.unwrap();
    make_response(&json_data)
}
