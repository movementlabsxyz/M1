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
pub struct AccountParams {
    pub data: Option<String>,
    pub start: Option<usize>,
    pub limit: Option<usize>,
    pub ledger_version: Option<String>,
}
pub async fn get_accounts_transactions(Path(address): Path<String>) -> impl IntoResponse {
    println!("get_accounts_transactions");
    let json_data = request_to_subnet_ext::<serde_json::Value>("getAccountsTransactions", Some(json!({ "data": address }))).await.unwrap();
    make_response(&json_data)
}

pub async fn get_account(
    Path(address): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(mut params) = params.unwrap_or_default();
    params.data = Some(address);

    println!("get_account");
    let json_data = request_to_subnet_ext::<AccountParams>("getAccount", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_account_resources(
    Path(address): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(mut params) = params.unwrap_or_default();
    params.data = Some(address);

    println!("get_account_resources");
    let json_data = request_to_subnet_ext::<AccountParams>("getAccountResources", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_account_modules(
    Path(address): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(mut params) = params.unwrap_or_default();
    params.data = Some(address);

    println!("get_account_modules");
    let json_data = request_to_subnet_ext::<AccountParams>("getAccountResources", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_account_resources_state(
    Path(address): Path<String>,
    Path(resource_type): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let params = json!({
      "account": address,
      "resource": resource_type,
      "ledger_version": params.ledger_version
    });

    println!("get_account_resources_state");
    let json_data = request_to_subnet_ext::<serde_json::Value>("getAccountResourcesState", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_account_modules_state(
    Path(address): Path<String>,
    Path(module_name): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let params = json!({
      "account": address,
      "resource": module_name,
      "ledger_version": params.ledger_version
    });

    println!("get_account_modules_state");
    let json_data = request_to_subnet_ext::<serde_json::Value>("getAccountModulesState", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_events_by_creation_number(
    Path(address): Path<String>,
    Path(creation_number): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let params = json!({
      "address": address,
      "creation_number": creation_number,
      "start": params.start,
      "limit": params.limit
    });

    println!("get_events_by_creation_number");
    let json_data = request_to_subnet_ext::<serde_json::Value>("getEventsByCreationNumber", Some(params)).await.unwrap();
    make_response(&json_data)
}

pub async fn get_events_by_event_handle(
    Path(address): Path<String>,
    Path(event_handle): Path<String>,
    Path(field_name): Path<String>,
    params: Option<Query<AccountParams>>,
) -> impl IntoResponse {
    let Query(params) = params.unwrap_or_default();
    let params = json!({
      "address": address,
      "event_handle": event_handle,
      "field_name": field_name,
      "start": params.start,
      "limit": params.limit
    });

    println!("get_events_by_event_handle");
    let json_data = request_to_subnet_ext::<serde_json::Value>("getEventsByEventHandle", Some(params)).await.unwrap();
    make_response(&json_data)
}
